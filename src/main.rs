#![feature(try_trait)]

use chrono::*;
use chrono_tz::*;
use failure::*;
use lazy_static::*;
use icalendar::*;
use regex::Regex;
use std::{
    fmt::Display,
    fs::*,
    io::{
        BufRead,
        BufReader,
        stdin,
    }
};

fn main() -> Result<(), Error> {
    let team_name = "Real Portland".to_string().to_uppercase();
    let calendar = schedule_to_ical(stdin().lock(), &team_name)?;
    calendar.print().context("Calendar could not be printed")?;
    Ok(())
}

fn schedule_to_ical(input: impl BufRead, team_name: &str) -> Result<Calendar, Error> {
    let mut calendar = Calendar::new();

    for line in input.lines() {
        if let Some(game) = parse_schedule_line(&line.context("Line could not be read")?, team_name)? {
            calendar.push(game_to_event(game));
        }
    }

    Ok(calendar)
}

fn game_to_event<'a>(game: Game<'a, chrono_tz::Tz>) -> Event {
    Event::new()
        .summary(&(game.home.to_string() + " (home) vs. " + game.away))
        .description("Home team brings ball & all colors")
        .location("Portland Indoor Soccer\n418 SE Main St.\nPortland, OR 97214")
        .starts(game.date)
        .ends(game.date + Duration::minutes(44+2))
        .done()
}

struct Game<'a, Tz: TimeZone> 
where Tz::Offset: Display {
    home: &'a str,
    away: &'a str,
    date: DateTime<Tz>,
}

fn parse_schedule_line<'a>(line: &'a str, team_name: &'a str) -> Result<Option<Game<'a, chrono_tz::Tz>>, Error> {
    lazy_static! {
        static ref game_regex: Regex = Regex::new(r"^[A-Z]{3} [A-Z]{3} [0-9 ]{2} +[0-9 ]{2}:[0-9]{2} [AP]M  (.*) vs (.*)$").unwrap();
    }

    if let Some(groups) = game_regex.captures(&line) {
        let home = groups.get(1).expect("Game regex missing capture #1").as_str().trim();
        let away = groups.get(2).expect("Game regex missing capture #2").as_str().trim();
        // TODO: filter elsewhere?
        if home == team_name || away == team_name {
            // TODO: test fall schedule for new year's rollover edge case
            let line = line.split_at(22).0.to_string() + "2019";
            let date = US::Pacific.datetime_from_str(&line, "%a %b %e %I:%M %p %Y").with_context(|e| {format!("Error parsing datetime string: {}", e)})?;
            return Ok(Some(Game { home, away, date }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datetime_parse() -> Result<(), Error> {
        let mut to_parse = "SUN Jan 20    7:50 PM".to_string();
        // Year isn't specified in string, but must be for parsing
        to_parse.push_str(" 2019");

        let dt = US::Pacific.datetime_from_str(&to_parse, "%a %b %e    %I:%M %p %Y").with_context(|e| {format!("Error parsing datetime string: {}", e)})?;
        assert_eq!("2019-01-20T19:50:00-08:00", dt.to_rfc3339());

        Ok(())
    }

    #[test]
    fn datetime_fewer_spaces_parse() -> Result<(), Error> {
        let mut to_parse = "SUN Jan 20 7:50 PM".to_string();
        // Year isn't specified in string, but must be for parsing
        to_parse.push_str(" 2019");

        let dt = US::Pacific.datetime_from_str(&to_parse, "%a %b %e    %I:%M %p %Y").with_context(|e| {format!("Error parsing datetime string: {}", e)})?;
        assert_eq!("2019-01-20T19:50:00-08:00", dt.to_rfc3339());

        Ok(())
    }

    #[test]
    fn convert_test_schedule_stdin() -> Result<(), Error> {
        let input = "test/div3b/input.txt";
        let expected = "test/div3b/expected.ical";
        let team_name = "Real Portland".to_string().to_uppercase();

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
}
