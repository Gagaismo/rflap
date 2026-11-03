//! Minimal XML element tree with the pretty writer JFLAP uses (`DOMPrettier` + `Transformer`).

use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};

use super::error::JffError;

/// A child of an element: another element or a comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Comment(String),
}

/// An XML element. `text` is the character data directly inside it; elements that
/// contain other elements drop whitespace-only text (indentation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(name: &str) -> Self {
        Element {
            name: name.to_string(),
            attributes: Vec::new(),
            text: String::new(),
            children: Vec::new(),
        }
    }

    /// An element holding only text.
    pub fn with_text(name: &str, text: &str) -> Self {
        let mut e = Element::new(name);
        e.text = text.to_string();
        e
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn set_attr(&mut self, name: &str, value: &str) {
        match self.attributes.iter_mut().find(|(k, _)| k == name) {
            Some((_, v)) => *v = value.to_string(),
            None => self.attributes.push((name.to_string(), value.to_string())),
        }
    }

    /// Appends a child element.
    pub fn push(&mut self, child: Element) {
        self.children.push(Node::Element(child));
    }

    pub fn push_comment(&mut self, text: &str) {
        self.children.push(Node::Comment(text.to_string()));
    }

    /// Child elements in order.
    pub fn elements(&self) -> impl Iterator<Item = &Element> + '_ {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Comment(_) => None,
        })
    }

    /// Child elements with the given name, in order.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.elements().filter(move |e| e.name == name)
    }

    /// First child element with the given name.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|e| e.name == name)
    }

    /// Text of the first child element with the given name.
    pub fn child_text(&self, name: &str) -> Option<&str> {
        self.child(name).map(|e| e.text.as_str())
    }

    /// First descendant (document order, self excluded) with the given name
    /// (DOM `getElementsByTagName(name).item(0)`).
    pub fn descendant(&self, name: &str) -> Option<&Element> {
        self.elements().find_map(|e| {
            if e.name == name {
                Some(e)
            } else {
                e.descendant(name)
            }
        })
    }
}

fn bad(e: impl std::fmt::Display) -> JffError {
    JffError::BadXml(e.to_string())
}

fn start_element(e: &BytesStart<'_>) -> Result<Element, JffError> {
    let name = e.name().as_ref().to_string();
    let mut el = Element::new(&name);
    for attr in e.attributes() {
        let attr = attr.map_err(bad)?;
        let key = attr.key.as_ref().to_string();
        let value = unescape(&attr.value).map_err(bad)?.into_owned();
        el.attributes.push((key, value));
    }
    Ok(el)
}

/// Parses a document and returns its root element.
pub fn parse_document(xml: &str) -> Result<Element, JffError> {
    // XML parsers turn CR LF and lone CR into LF.
    let xml = xml.replace("\r\n", "\n").replace('\r', "\n");
    let mut reader = Reader::from_str(&xml);
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    loop {
        let event = reader.read_event().map_err(bad)?;
        match event {
            Event::Start(e) => {
                if root.is_some() {
                    return Err(bad("more than one root element"));
                }
                stack.push(start_element(&e)?);
            }
            Event::Empty(e) => {
                let el = start_element(&e)?;
                close(el, &mut stack, &mut root)?;
            }
            Event::End(_) => {
                let el = stack.pop().ok_or_else(|| bad("unexpected end tag"))?;
                close(el, &mut stack, &mut root)?;
            }
            Event::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.into_inner());
                }
            }
            Event::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.into_inner());
                }
            }
            Event::GeneralRef(r) => {
                let reference = format!("&{};", r.into_inner());
                let value = unescape(&reference).map_err(bad)?;
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&value);
                }
            }
            Event::Comment(t) => {
                if let Some(top) = stack.last_mut() {
                    top.children
                        .push(Node::Comment(t.into_inner().into_owned()));
                }
            }
            Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
            Event::Eof => break,
        }
    }
    if !stack.is_empty() {
        return Err(bad("the document ends inside an open element"));
    }
    root.ok_or_else(|| bad("the document has no root element"))
}

fn close(
    mut el: Element,
    stack: &mut [Element],
    root: &mut Option<Element>,
) -> Result<(), JffError> {
    if el.elements().next().is_some() && el.text.trim().is_empty() {
        el.text.clear();
    }
    match stack.last_mut() {
        Some(parent) => parent.children.push(Node::Element(el)),
        None => {
            if root.is_some() {
                return Err(bad("more than one root element"));
            }
            *root = Some(el);
        }
    }
    Ok(())
}

fn escape_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

fn escape_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

fn write_element(el: &Element, indent: usize, out: &mut String) {
    out.push('<');
    out.push_str(&el.name);
    for (k, v) in &el.attributes {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        escape_attr(v, out);
        out.push('"');
    }
    if el.children.is_empty() && el.text.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    escape_text(&el.text, out);
    for child in &el.children {
        out.push('\n');
        out.push_str(&"\t".repeat(indent + 1));
        match child {
            Node::Element(e) => write_element(e, indent + 1, out),
            Node::Comment(c) => {
                out.push_str("<!--");
                out.push_str(c);
                out.push_str("-->");
            }
        }
    }
    if !el.children.is_empty() {
        out.push('\n');
        out.push_str(&"\t".repeat(indent));
    }
    out.push_str("</");
    out.push_str(&el.name);
    out.push('>');
}

/// Serializes a document the way JFLAP does: XML declaration, the
/// `Created with JFLAP 6.4.` comment and the root, one tab per level, LF line ends,
/// no trailing newline.
pub fn write_document(root: &Element) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?><!--Created with JFLAP 6.4.-->",
    );
    write_element(root, 0, &mut out);
    out
}
