use icalendar::*;
use chrono::*;
use chrono_tz::*;
use failure::*;
use regex::Regex;
use std::io::BufRead;

fn main() {
    let team_name = "Real Portland".to_string().to_uppercase();
    // todo: trim results
    let r = Regex::new(r"^[A-Z]{3} [A-Z]{3} [0-9 ]{2} +[0-9 ]{2}:[0-9]{2} [AP]M  (.*) vs (.*)$").unwrap();
    std::io::stdin().lock().lines().for_each(|i| {
        let i = i.unwrap();
        if r.is_match(&i) {
            println!("{}", i);
        }
    });
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
