use std::borrow::Cow;
/// use GivenNameToken::*;
/// "John R L" == &[Name("John"), Initial("R"), Initial("L")]
/// "Jean-Luc K" = &[Name("Jean"), HyphenSegment("Luc"), Initial("K")]
/// "R. L." = &[Initial("R"), Initial("L")]
///
#[derive(Clone, Copy, PartialEq, Debug)]
enum GivenNameToken<'n> {
    Name(&'n str),
    Initial(&'n str),
    HyphenSegment(&'n str),
    Other(&'n str),
}

use self::GivenNameToken::*;

pub fn initialize<'n>(
    given_name: &'n str,
    initialize: bool,
    with: Option<&str>,
    initial_hyphens: bool,
) -> Cow<'n, str> {
    if let Some(with) = with {
        let mut build = String::new();
        let mut first = true;
        let mut last_was_initial = false;

        let mut process_token = |token: GivenNameToken| {
            match token {
                Name(ref n) => {
                    if initialize {
                        if !first && !last_was_initial {
                            build.push(' ');
                        }
                        build.push(n.chars().nth(0).unwrap());
                        build.push_str(with);
                        last_was_initial = true;
                    } else {
                        if !first {
                            build.push(' ');
                        }
                        build.push_str(n);
                        last_was_initial = false;
                    }
                }
                Initial(ref n) => {
                    if !first && !last_was_initial {
                        build.push(' ');
                    }
                    build.push_str(n);
                    build.push_str(with);
                    last_was_initial = true;
                }
                HyphenSegment(ref n) => {
                    if initialize {
                        if initial_hyphens {
                            build.push('-');
                        }
                        build.push(n.chars().nth(0).unwrap());
                        build.push_str(with);
                        last_was_initial = true;
                    } else {
                        build.push('-');
                        build.push_str(n);
                        last_was_initial = false;
                    }
                }
                Other(ref n) => {
                    if !first {
                        build.push_str(" ");
                    }
                    build.push_str(n);
                    last_was_initial = false;
                }
            }
            first = false;
            // slightly hacky, but you may want to disable adding extra spaces so
            // initialize-with=". " doesn't produce "W. A.  Mozart"
        };

        for word in given_name.split(&[' ', '.'][..]) {
            if word == "" {

            } else if !word.chars().nth(0).unwrap().is_uppercase() {
                // 'not uppercase' also includes uncased code points like Chinese or random punctuation
                process_token(Other(word));
            } else if word.len() == 1 && word.chars().all(|c| c.is_uppercase()) {
                process_token(Initial(word));
            } else {
                let mut segs = word.split('-');
                if let Some(first) = segs.next() {
                    process_token(Name(first));
                    for seg in segs {
                        process_token(HyphenSegment(seg));
                    }
                }
            }
        }
        Cow::Owned(build)
    } else {
        Cow::Borrowed(given_name)
    }
}

#[test]
fn test_initialize_full() {
    fn init<'a>(given_name: &'a str) -> Cow<'a, str> {
        initialize(given_name, true, Some(""), false)
    }
    assert_eq!(init("John R L"), "JRL");
    assert_eq!(init("Jean-Luc K"), "JLK");
    assert_eq!(init("R. L."), "RL");
    assert_eq!(init("R L"), "RL");
    assert_eq!(init("John R.L."), "JRL");
    assert_eq!(init("John R L de Bortoli"), "JRL de B");
}

#[test]
fn test_initialize_hyphen() {
    fn init<'a>(given_name: &'a str) -> Cow<'a, str> {
        initialize(given_name, true, Some("."), true)
    }
    assert_eq!(init("John R L"), "J.R.L.");
    assert_eq!(init("Jean-Luc K"), "J.-L.K.");
    assert_eq!(init("R. L."), "R.L.");
    assert_eq!(init("R L"), "R.L.");
    assert_eq!(init("John R.L."), "J.R.L.");
    assert_eq!(init("John R L de Bortoli"), "J.R.L. de B.");
    assert_eq!(init("好 好"), "好 好");
}

#[test]
fn test_initialize_normal() {
    fn init<'a>(given_name: &'a str) -> Cow<'a, str> {
        initialize(given_name, false, Some("."), true)
    }
    assert_eq!(init("John R L"), "John R.L.");
    assert_eq!(init("Jean-Luc K"), "Jean-Luc K.");
    assert_eq!(init("R. L."), "R.L.");
    assert_eq!(init("R L"), "R.L.");
    assert_eq!(init("John R.L."), "John R.L.");
    assert_eq!(init("John R L de Bortoli"), "John R.L. de Bortoli");
    assert_eq!(init("好 好"), "好 好");
}
