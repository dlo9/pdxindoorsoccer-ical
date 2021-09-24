use caseless::default_caseless_match_str;
use chrono::*;
use chrono_tz::*;
use encoding_rs::*;
use anyhow::{Context, Result};
use heck::TitleCase;
use icalendar::*;
use lazy_static::*;
use regex::Regex;
use std::{
    fmt::Display,
    fs::File,
    io::{stdin, BufRead, BufReader},
    path::PathBuf,
};
use structopt::*;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pdxindoorsoccer-ical",
    rename_all = "kebab-case",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
struct Args {
    /// Output ical file. If not specified, stdout is used.
    // TODO: unconnected
    #[structopt(short = "o", long)]
    output: Option<PathBuf>,

    /// Dry run
    #[structopt(short = "n", long)]
    dry_run: bool,

    /// Input text file. If not specified, stdin is used.
    // TODO: accept - as stdin?
    #[structopt(short = "i", long)]
    input: Option<PathBuf>,

    #[structopt(short = "u", long, conflicts_with = "input")]
    url: Option<String>,

    /// Team name to filter to
    // TODO: Make optional, no filter doesn't filter
    #[structopt(short = "t", long)]
    team_name: String,
}

#[paw::main]
fn main(args: Args) -> Result<()> {
    futures::executor::block_on(main_async(args))
}

async fn main_async(args: Args) -> Result<()> {
    // TODO: do a single function call, but instead return here different objects all
    // impl BufRead?
    let calendar = if let Some(path) = args.input {
        // TODO: decode?
        schedule_to_ical(BufReader::new(File::open(path)?), &args.team_name)?
    } else if let Some(url) = args.url {
        let response = reqwest::get(&url).await?.bytes().await?;
        let decoded = WINDOWS_1252.decode(&response).0;
        schedule_to_ical(decoded.as_ref().as_bytes(), &args.team_name)?
    } else {
        schedule_to_ical(stdin().lock(), &args.team_name)?
    };

    calendar.print().context("Calendar could not be printed")?;
    Ok(())
}

fn schedule_to_ical(input: impl BufRead, team_name: &str) -> Result<Calendar> {
    let mut calendar = Calendar::new();
    let mut year = 0;
    let mut last_month = 0;

    for line in input.lines() {
        let line = line.context("Line could not be read")?;

        if year == 0 {
            if let Some(parsed_year) = parse_year_line(&line) {
                year = parsed_year;
            }
        } else if let Some(game) = parse_game_line(&line, year)? {
            // Handle year rollover
            if game.datetime.date().month() < last_month {
                year += 1;
            }

            last_month = game.datetime.date().month();

            if default_caseless_match_str(game.home, team_name)
                || default_caseless_match_str(game.away, team_name)
            {
                let home = game.home.fix_case();
                let away = game.away.fix_case();
                let game = Game {
                    home: &home,
                    away: &away,
                    ..game
                };
                calendar.push(Event::from(game));
            }
        }
    }

    Ok(calendar)
}

trait StrExt {
    fn fix_case(&self) -> String;
}

impl StrExt for &str {
    fn fix_case(&self) -> String {
        fc_to_uppercase(self.to_title_case())
    }
}

/// Forces the casing of FC to uppercase
fn fc_to_uppercase(s: String) -> String {
    lazy_static! {
        static ref FC_REGEX: Regex = Regex::new(r#"(.*\b)((?i)\w*fc)(\b.*)"#).unwrap();
    }

    FC_REGEX
        .captures(&s)
        .map(|c| {
            format!(
                "{}{}{}",
                c.get(1).unwrap().as_str(),
                c.get(2).unwrap().as_str().to_uppercase(),
                c.get(3).unwrap().as_str()
            )
        })
        .unwrap_or(s)
}

struct Game<'a, Tz: TimeZone>
where
    Tz::Offset: Display,
{
    home: &'a str,
    away: &'a str,
    datetime: DateTime<Tz>,
}

impl<'a> From<Game<'a, chrono_tz::Tz>> for Event {
    fn from(
        Game {
            home,
            away,
            datetime,
        }: Game<'a, chrono_tz::Tz>,
    ) -> Self {
        let datetime: DateTime<Utc> = datetime.with_timezone(&Utc);
        Event::new()
            .summary(&(home.to_string() + " (home) vs. " + away))
            .description("Home team brings ball & all colors")
            .location("Portland Indoor Soccer\n418 SE Main St.\nPortland\\, OR 97214")
            .starts(datetime)
            .ends(datetime + Duration::minutes(44 + 2))
            .done()
    }
}

fn parse_year_line<'a>(line: &'a str) -> Option<u16> {
    lazy_static! {
        static ref YEAR_REGEX: Regex = Regex::new(r" CUP ([0-9]{4})\s*$").unwrap();
    }

    YEAR_REGEX.captures(&line).and_then(|groups| {
        Some(
            groups
                .get(1)
                .expect("Year regex missing capture #1")
                .as_str()
                .parse::<u16>()
                .expect("Year int parse failed"),
        )
    })
}

fn parse_game_line<'a>(line: &'a str, year: u16) -> Result<Option<Game<'a, chrono_tz::Tz>>> {
    lazy_static! {
        static ref GAME_REGEX: Regex =
            Regex::new(r"^[A-Z]{3} ([A-Z]{3} [0-9 ]{2} +[0-9 ]{2}:[0-9]{2} [AP]M)  (.*) vs (.*)$")
                .unwrap();
    }

    if let Some(groups) = GAME_REGEX.captures(&line) {
        let datetime = groups
            .get(1)
            .expect("Game regex missing capture #1")
            .as_str()
            .trim()
            .to_string()
            + " "
            + &year.to_string();
        let home = groups
            .get(2)
            .expect("Game regex missing capture #2")
            .as_str()
            .trim();
        let away = groups
            .get(3)
            .expect("Game regex missing capture #3")
            .as_str()
            .trim();

        let datetime = US::Pacific
            .datetime_from_str(&datetime, "%b %e %I:%M %p %Y")
            .with_context(|| format!("Failed to parse datetime string: {}", &datetime))?;
        return Ok(Some(Game {
            home,
            away,
            datetime,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{fs::*, io::BufReader};

    #[test]
    fn datetime_parse() -> Result<()> {
        let mut datetime = "SUN Jan 20    7:50 PM".to_string();
        // Year isn't specified in string, but must be for parsing
        datetime.push_str(" 2019");

        let dt = US::Pacific
            .datetime_from_str(&datetime, "%a %b %e    %I:%M %p %Y")
            .with_context(|| format!("Failed to parse datetime string: {}", &datetime))?;
        assert_eq!("2019-01-20T19:50:00-08:00", dt.to_rfc3339());

        Ok(())
    }

    #[test]
    fn datetime_fewer_spaces_parse() -> Result<()> {
        let mut datetime = "SUN Jan 20 7:50 PM".to_string();
        // Year isn't specified in string, but must be for parsing
        datetime.push_str(" 2019");

        let dt = US::Pacific
            .datetime_from_str(&datetime, "%a %b %e    %I:%M %p %Y")
            .with_context(|| format!("Failed to parse datetime string: {}", &datetime))?;
        assert_eq!("2019-01-20T19:50:00-08:00", dt.to_rfc3339());

        Ok(())
    }

    #[test]
    fn convert_test_schedule_stdin_same_year() -> Result<()> {
        let input = "test/div3b/input.txt";
        let expected = "test/div3b/expected.ical";
        let team_name = "Real Portland".to_string().to_uppercase();
        convert_test_schedule_stdin(input, expected, &team_name)
    }

    #[test]
    fn convert_new_schedule_unknown_issue() -> Result<()> {
        let input = "test/winter/input.txt";
        let expected = "test/winter/expected.ical";
        let team_name = "Friend Hotel".to_string().to_uppercase();
        convert_test_schedule_stdin(input, expected, &team_name)
    }

    #[test]
    fn convert_test_schedule_stdin_year_boundary() -> Result<()> {
        let input = "test/fall/input.txt";
        let expected = "test/fall/expected.ical";
        let team_name = "Hyventus".to_string().to_uppercase();
        convert_test_schedule_stdin(input, expected, &team_name)
    }

    #[test]
    fn parse_year_line_fall() {
        assert_eq!(
            Some(2018),
            parse_year_line(
                "                          SECOND FALL CUP 2018                             "
            )
        )
    }

    #[test]
    fn parse_year_line_winter() {
        assert_eq!(Some(2020), parse_year_line("		           WINTER CUP 2020"))
    }

    fn convert_test_schedule_stdin(
        input: &str,
        expected: &str,
        team_name: &str,
    ) -> Result<()> {
        let input = BufReader::new(File::open(input)?);

        let calendar = schedule_to_ical(input, &team_name);

        let expected = read_to_string(expected)?;
        let actual = calendar?.to_string();

        // Sort lines since to_string isn't deterministically ordered
        // Also strip randomized UID & DTSTAMP
        let mut expected = expected
            .split("\r\n")
            .filter(|i| !(i.starts_with("DTSTAMP") || i.starts_with("UID")))
            .collect::<Vec<&str>>();
        expected.sort_unstable();
        let mut actual = actual
            .split("\r\n")
            .filter(|i| !(i.starts_with("DTSTAMP") || i.starts_with("UID")))
            .collect::<Vec<&str>>();
        actual.sort_unstable();

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn fc_to_uppercase_at_start() {
        let s = "fc is at the start";
        let expected = "FC is at the start";
        assert_eq!(expected, fc_to_uppercase(s.into()));

        let s = "nrfc is at the start";
        let expected = "NRFC is at the start";
        assert_eq!(expected, fc_to_uppercase(s.into()));

        let s = "fcnr is at the start";
        let expected = s.to_string();
        assert_eq!(expected, fc_to_uppercase(s.into()));
    }

    #[test]
    fn fc_to_uppercase_at_end() {
        let s = "At the end is fc";
        let expected = "At the end is FC";
        assert_eq!(expected, fc_to_uppercase(s.into()));

        let s = "At the end is nrfc";
        let expected = "At the end is NRFC";
        assert_eq!(expected, fc_to_uppercase(s.into()));

        let s = "At the end is fcnr";
        let expected = s.to_string();
        assert_eq!(expected, fc_to_uppercase(s.into()));
    }

    #[test]
    fn fc_to_uppercase_in_middle() {
        let s = "We have fc in the middle";
        let expected = "We have FC in the middle";
        assert_eq!(expected, fc_to_uppercase(s.into()));

        let s = "We have nrfc in the middle";
        let expected = "We have NRFC in the middle";
        assert_eq!(expected, fc_to_uppercase(s.into()));

        let s = "We have fcnr in the middle";
        let expected = s.to_string();
        assert_eq!(expected, fc_to_uppercase(s.into()));
    }
}
