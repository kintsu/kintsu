use crate::tokens::{ToTokens, toks::Token};

use super::FormatConfig;

#[derive(Default, Clone, Debug)]
pub struct Printer {
    pub buf: String,
    pub indent_level: usize,
    pub paren_depth: usize,
    pub brace_depth: usize,
    pub bracket_depth: usize,
    pub cfg: FormatConfig,
}

impl Printer {
    pub fn new(cfg: &FormatConfig) -> Self {
        Self {
            buf: String::with_capacity(1024),
            indent_level: 0,
            paren_depth: 0,
            brace_depth: 0,
            bracket_depth: 0,
            cfg: cfg.clone(),
        }
    }

    pub fn word(
        &mut self,
        s: &str,
    ) {
        self.buf.push_str(s);
    }

    pub fn token(
        &mut self,
        t: &Token,
    ) {
        self.buf.push_str(&format!("{t}"));
    }

    pub fn space(&mut self) {
        self.buf.push(' ');
    }

    pub fn add_newline(&mut self) {
        self.buf.push('\n');
        self.add_indent();
    }

    pub fn add_indent(&mut self) {
        if self.cfg.indent_with_tabs {
            for _ in 0..self.indent_level {
                self.buf.push('\t');
            }
        } else {
            for _ in 0..self.indent_level * self.cfg.indent_width {
                self.buf.push(' ');
            }
        }
    }

    pub fn open_block(&mut self) {
        self.token(&Token::LBrace);
        self.indent_level += 1;
        self.add_newline();
    }
    pub fn close_block(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
        self.add_newline();
        self.token(&Token::RBrace);
    }

    pub fn write<W: ToTokens + ?Sized>(
        &mut self,
        w: &W,
    ) {
        w.write(self);
    }

    pub fn write_comma_separated<T, I>(
        &mut self,
        items: I,
    ) where
        T: ToTokens,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator, {
        let iter = items.into_iter();
        let len = iter.len();
        for (idx, item) in iter.enumerate() {
            self.write(&item);
            if idx < len - 1 {
                self.token(&Token::Comma);
                self.add_newline();
            }
        }
    }

    pub fn write_comma_separated_inline<T, I>(
        &mut self,
        items: I,
    ) where
        T: ToTokens,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator, {
        let iter = items.into_iter();
        let len = iter.len();
        for (idx, item) in iter.enumerate() {
            self.write(&item);
            if idx < len - 1 {
                self.token(&Token::Comma);
                self.space();
            }
        }
    }
}

pub fn print_tokens(
    tokens: &[crate::defs::Spanned<crate::tokens::toks::Token>],
    cfg: &FormatConfig,
) -> String {
    let mut p = Printer::new(cfg);
    for t in tokens {
        p.token(&t.value);
    }
    p.buf
}

pub fn print_ast(
    ast: &crate::ast::AstStream,
    cfg: &FormatConfig,
) -> String {
    let mut p = Printer::new(cfg);
    p.write(ast);
    p.buf
}
