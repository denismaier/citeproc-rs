mod pandoc;
mod markdown;
mod plain;

pub use self::pandoc::Pandoc;
pub use self::plain::PlainText;
pub use self::markdown::Markdown;

use crate::style::element::Formatting;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Output<T> {
    pub citations: Vec<T>,
    pub bibliography: Vec<T>,
    pub citation_ids: Vec<String>,
}

pub trait OutputFormat<T, O: Serialize> {
    // affixes are not included in the formatting on a text node.
    // affixes are converted into text nodes themselves, with Formatting::default() passed.
    // http://docs.citationstyles.org/en/stable/specification.html#affixes
    fn text_node(&self, s: &str, formatting: &Formatting) -> T;
    fn group(&self, nodes: &[T], delimiter: &str, formatting: &Formatting) -> T;
    fn output(&self, intermediate: T) -> O;

    fn plain(&self, s: &str) -> T {
        self.text_node(s, &Formatting::default())
    }
}

#[cfg(test)]
mod test {

    use crate::style::element::Formatting;

    use super::Format;
    use super::PlainText;

    // #[test]
    // fn markdown() {
    //     let f = Markdown::new();
    //     let o = f.text_node("hi", &Formatting::italic());
    //     let o2 = f.text_node("mom", &Formatting::bold());
    //     let o3 = f.group(&[o, o2], " ", &Formatting::italic());
    //     let serialized = serde_json::to_string(&o3).unwrap();
    //     assert_eq!(serialized, "\"_hi **mom**_\"");
    // }

    #[test]
    fn plain() {
        let f = PlainText::new();
        let o = f.text_node("hi", &Formatting::italic());
        let o2 = f.text_node("mom", &Formatting::default());
        let o3 = f.group(&[o, o2], " ", &Formatting::italic());
        let serialized = serde_json::to_string(&o3).unwrap();
        assert_eq!(serialized, "\"hi mom\"");
    }

}
