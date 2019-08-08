use crate::parser::{Parser, Token};

pub struct AttributeDeserializer<'a, 'de: 'a> {
    parser: &'a mut Parser<'de>,
    last_key: Vec<u8>,
}

impl<'a, 'de> AttributeDeserializer<'a, 'de> {
    pub fn new(parser: &'a mut Parser<'de>) -> Self {
        AttributeDeserializer {
            parser,
            last_key: Vec::new(),
        }
    }

    pub fn accept(
        mut self,
        mut visitor: impl AttributeVisitor<'de>,
        outer_open_key: Option<&[u8]>,
    ) -> Option<()> {
        loop {
            match self.parser.next() {
                Some(Token::Attr { key, value }) => {
                    if self.last_key >= key {
                        return None;
                    }
                    self.last_key.clear();
                    self.last_key.extend(&key);
                    visitor.visit_attribute(key, value)?;
                }
                Some(Token::Open { open_key: first_open_key }) => {
                    let de = ChildrenDeserializer::new(self.parser);
                    return de.accept(
                        first_open_key,
                        outer_open_key,
                        visitor.start_children(),
                    );
                }
                // TODO duplication
                Some(Token::Close { close_key }) => {
                    return outer_open_key
                        .filter(|&open_key| open_key == close_key.as_slice())
                        .map(drop);
                }
                None => {
                    return if outer_open_key.is_some() { None } else { Some(()) };
                }
            }
        }
    }
}

struct ChildrenDeserializer<'a, 'de: 'a> {
    parser: &'a mut Parser<'de>,
}

impl<'a, 'de> ChildrenDeserializer<'a, 'de> {
    fn new(parser: &'a mut Parser<'de>) -> Self {
        ChildrenDeserializer { parser }
    }

    fn accept(
        mut self,
        first_open_key: Vec<u8>,
        outer_open_key: Option<&[u8]>,
        mut visitor: impl ChildrenVisitor<'de>,
    ) -> Option<()> {
        self.accept_child(&first_open_key, &mut visitor)?;
        loop {
            match self.parser.next() {
                Some(Token::Attr { .. }) => {
                    return None;
                }
                Some(Token::Open { open_key }) => {
                    self.accept_child(&open_key, &mut visitor)?;
                }
                // TODO duplication
                Some(Token::Close { close_key }) => {
                    return outer_open_key
                        .filter(|&open_key| open_key == close_key.as_slice())
                        .map(drop);
                }
                None => {
                    return if outer_open_key.is_some() { None } else { Some(()) };
                }
            }
        }
    }

    fn accept_child(
        &mut self,
        open_key: &[u8],
        visitor: &mut impl ChildrenVisitor<'de>,
    ) -> Option<()> {
        let de = AttributeDeserializer::new(self.parser);
        de.accept(visitor.visit_child(&open_key)?, Some(&open_key))
    }
}

pub trait AttributeVisitor<'de> {
    type ChildrenVisitor: ChildrenVisitor<'de>;
    fn visit_attribute(&mut self, key: Vec<u8>, value: Vec<u8>) -> Option<()>;
    fn start_children(self) -> Self::ChildrenVisitor;
}

pub trait ChildrenVisitor<'de> {
    type AttributeVisitor: AttributeVisitor<'de>;
    fn visit_child(&mut self, open_key: &[u8]) -> Option<Self::AttributeVisitor>;
}
