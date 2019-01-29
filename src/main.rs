use icalendar::*;
use chrono::*;
use chrono_tz::*;
use failure::*;
use regex::Regex;
use std::io::BufRead;

fn main() -> Result<(), Error> {
    let team_name = "Real Portland".to_string().to_uppercase();

    let r = Regex::new(r"^[A-Z]{3} [A-Z]{3} [0-9 ]{2} +[0-9 ]{2}:[0-9]{2} [AP]M  (.*) vs (.*)$").unwrap();
    let mut calendar = Calendar::new();

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let groups = r.captures(&line);
        if groups.is_some() {
            let groups = groups.unwrap();
            let home = groups.get(1).unwrap().as_str().trim();
            let away = groups.get(2).unwrap().as_str().trim();
            if home == team_name || away == team_name {
                // TODO: test fall schedule for new year's rollover edge case
                let line = line.split_at(22).0.to_string() + "2019";
                //println!("{}", line);
                let date = US::Pacific.datetime_from_str(&line, "%a %b %e %I:%M %p %Y").with_context(|e| {format!("Error parsing datetime string: {}", e)})?;

                let event = Event::new()
                    .summary(&(home.to_string() + " (home) vs." + away))
                    .description("Home team brings ball & all colors")
                    // TODO: location?
                    // TODO: set busy
                    .starts(date)
                    .ends(date + Duration::minutes(44+2))
                    .done();

                calendar.push(event);
            }
        }
    }

    calendar.print().unwrap();
    Ok(())
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

    fn datetime_fewer_spaces_parse() -> Result<(), Error> {
        let mut to_parse = "SUN Jan 20 7:50 PM".to_string();
        // Year isn't specified in string, but must be for parsing
        to_parse.push_str(" 2019");

        let dt = US::Pacific.datetime_from_str(&to_parse, "%a %b %e    %I:%M %p %Y").with_context(|e| {format!("Error parsing datetime string: {}", e)})?;
        assert_eq!("2019-01-20T19:50:00-08:00", dt.to_rfc3339());
        
        Ok(())
    }
}
