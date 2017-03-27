use std::collections::HashMap;
use regex::Regex;

const WHITESPACE: &'static str = r#"\s*"#;
const MATCH_NAME: &'static str = "([^<>]+)";
const MATCH_EMAIL: &'static str = "<([^<>]+)>";

// 'PNAME' =  Proper Name
// 'PEMAIL =  Proper Email
// 'CNAME' =  Commit Name
// 'CEMAIL' = Commit Email
lazy_static! {
    static ref PNAME_CEMAIL:             Regex = Regex::new(format!(r#"^{}{} {}\s*(#.*)?$"#,       WHITESPACE, MATCH_NAME,  MATCH_EMAIL).as_str()).unwrap();
    static ref PEMAIL_CEMAIL:            Regex = Regex::new(format!(r#"^{}{} {}\s*(#.*)?$"#,       WHITESPACE, MATCH_EMAIL, MATCH_EMAIL).as_str()).unwrap();
    static ref PNAME_PEMAIL_CEMAIL:      Regex = Regex::new(format!(r#"^{}{} {} {}\s*(#.*)?$"#,    WHITESPACE, MATCH_NAME,  MATCH_EMAIL, MATCH_EMAIL).as_str()).unwrap();
    static ref PNAME_PEMAIL_CNAME_CMAIL: Regex = Regex::new(format!(r#"^{}{} {} {} {}\s*(#.*)?"#, WHITESPACE, MATCH_NAME,  MATCH_EMAIL, MATCH_NAME, MATCH_EMAIL).as_str()).unwrap();
}

#[derive(Debug)]
struct Replacement {
    name: Option<String>,
    email: Option<String>
}

impl Replacement {
    fn name(name: String) -> Replacement {
        Replacement {
            name: Some(name),
            email: None
        }
    }

    fn email(email: String) -> Replacement {
        Replacement {
            name: None,
            email: Some(email)
        }
    }

    fn both(name: String, email: String) -> Replacement {
        Replacement {
            name: Some(name),
            email: Some(email)
        }
    }
}

pub struct Mailmap {
    email_map: HashMap<String, Replacement>, // Cache 'not found' to save a lookup
    name_email_map: HashMap<(String, String), (String, String)>
}

impl Mailmap {
    pub fn new(data: &str) -> Mailmap {
        let mut map = Mailmap {
            email_map: HashMap::new(),
            name_email_map: HashMap::new()
        };
        map.parse_map(data);
        map
    }

    fn parse_map(&mut self, data: &str) {
        macro_rules! capture {
            ($cap:expr, $n:expr) => {
                $cap.get($n).unwrap().as_str().to_owned()
            }
        }

        for line in data.split("\n") {
            if line.starts_with("#") {
                continue;
            }
            if let Some(cap) = PNAME_CEMAIL.captures(line) {
                //println!("PNAME_CEMAIL: '{}' {:?}", line, cap);
                self.email_map.insert(capture!(cap, 2).to_lowercase(), Replacement::name(capture!(cap, 1)));
            } else if let Some(cap) = PEMAIL_CEMAIL.captures(line) {
                //println!("PEMAIL_CEMAIL: '{}' {:?}", line, cap);
                self.email_map.insert(capture!(cap, 2).to_lowercase(), Replacement::email(capture!(cap, 1)));
            } else if let Some(cap) = PNAME_PEMAIL_CEMAIL.captures(line) {
                //println!("PNAME_PEMAIL_CEMAIL: '{}' {:?}", line, cap);
                self.email_map.insert(capture!(cap, 3).to_lowercase(), Replacement::both(capture!(cap, 1), capture!(cap, 2)));
            } else if let Some(cap) = PNAME_PEMAIL_CNAME_CMAIL.captures(line) {
                //println!("PNAME_PEMAIL_CNAME_CMAIL: '{}' {:?}", line, cap);
                self.name_email_map.insert((capture!(cap, 3).to_lowercase(), capture!(cap, 4).to_lowercase()), (capture!(cap, 1), capture!(cap, 2)));
            }
        }
    }

    pub fn map(&self, name: &str, email: &str) -> (String, String) {
        //println!("Lookup: ({}, {}) -> {:?}", name, email, self.email_map.get(email));
        let (lower_name, lower_email) = (name.to_lowercase(), email.to_lowercase()) ;
        if let Some(r) = self.email_map.get(&lower_email) {
            return (r.name.as_ref().map(|s| s.clone()).unwrap_or(name.to_owned()).clone(), r.email.as_ref().map(|s| s.clone()).unwrap_or(email.to_owned()).clone())
        }

        if let Some(&(ref r_name, ref r_email)) = self.name_email_map.get(&(lower_name.clone(), lower_email.clone())) {
            return (r_name.clone(), r_email.clone());
        } else {
            return (name.to_owned(), email.to_owned());
        }
    }
}

#[test]
fn test_mailmap() {

    macro_rules! check_map {
        ($map:expr, $name:expr, $email:expr, $expected_name:expr, $expected_email:expr) => {
            {
                let result: (String, String) = $map.map($name, $email);
                assert_eq!(($expected_name.to_owned(), $expected_email.to_owned()), result, "Expected to map '{},{}' to '{},{}', but instead mapped to '{},{}'", $name, $email, $expected_name, $expected_email, result.0, result.1);
            }
        }
     }

    let map_data = "Other Name <othername@example.com>
Three Four <threefour@example.com> # This is a comment
<properemail@example.com> <commitemail@example.com>
Bob Jones <bobjones@example.com> <fakejones@example.com>
# Comment line!
# Another comment
John Doe <johndoe@example.com> That Guy <thatguy@example.com>
Am Valid <valid@example.com> Also Valid <alsovalid@example.com> Not Valid <notvalid@example.com>";

    let m = Mailmap::new(map_data);

    // Authors not in the mailmap should be unchanged
    check_map!(m, "Not Here", "nothere@gmail.com", "Not Here", "nothere@gmail.com");

    // Ensure that non-mailmapped people with multiple names for one email work properly
    check_map!(m, "Kinda Here", "nothere@gmail.com", "Kinda Here", "nothere@gmail.com");

    // PNAME_CEMAIL rule
    check_map!(m, "Wrong name", "othername@example.com", "Other Name", "othername@example.com");
    check_map!(m, "One Two", "threefour@example.com", "Three Four", "threefour@example.com");

    // EMAIL_CEMAIL rule
    check_map!(m, "Aaron Hill", "commitemail@example.com", "Aaron Hill", "properemail@example.com");
    check_map!(m, "Random Name", "commitemail@example.com", "Random Name", "properemail@example.com");

    // PNAME_PEMAIL_CEMAIL
    check_map!(m, "Some Person", "fakejones@example.com", "Bob Jones", "bobjones@example.com");
    check_map!(m, "Someone Else", "fakejones@example.com", "Bob Jones", "bobjones@example.com");

    // PNAME_PEMAIL_CNAME_CMAIL
    check_map!(m, "That Guy", "thatguy@example.com", "John Doe", "johndoe@example.com");

    // Requires name and email to both match
    check_map!(m, "Other Guy", "thatguy@example.com", "Other Guy", "thatguy@example.com");
    check_map!(m, "That Guy", "blablah@example.com", "That Guy", "blablah@example.com");

    // Any name/email pairs after the second one should be ignored (for compatibility with git)
    check_map!(m, "Also Valid", "alsovalid@example.com", "Am Valid", "valid@example.com");
    check_map!(m, "Not Valid", "notvalid@example.com", "Not Valid", "notvalid@example.com");

    // Name/email comparisons are case-insensitive, but original case is preserved when replacement does not occur
    check_map!(m, "Wrong name", "OtHername@exAmple.com", "Other Name", "OtHername@exAmple.com");
    check_map!(m, "THAT guy", "ThATguy@examPle.com", "John Doe", "johndoe@example.com");
}
