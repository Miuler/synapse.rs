//! Tokenizador para texto con identificadores: separa palabras como `SimpleTokenizer` (letras y
//! números; `snake_case`, `kebab-case` y `a.b` ya quedan partidos) y, si una palabra es camelCase o
//! mezcla letras y números, emite además sus partes en la misma posición. Así
//! `AchOperationCrService` se encuentra entero y también por «operation» o «service».

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone, Default)]
pub struct IdentifierTokenizer {
    tokens: Vec<Token>,
}

pub struct IdentifierTokenStream<'a> {
    tokens: &'a mut [Token],
    index: usize,
}

impl Tokenizer for IdentifierTokenizer {
    type TokenStream<'a> = IdentifierTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> IdentifierTokenStream<'a> {
        self.tokens.clear();
        let mut position = 0;
        let mut start: Option<usize> = None;
        let flush = |tokens: &mut Vec<Token>, from: usize, to: usize, position: &mut usize| {
            let word = &text[from..to];
            tokens.push(Token {
                offset_from: from,
                offset_to: to,
                position: *position,
                text: word.to_string(),
                position_length: 1,
            });
            let parts = split_identifier(word);
            if parts.len() > 1 {
                for (a, b) in parts {
                    tokens.push(Token {
                        offset_from: from + a,
                        offset_to: from + b,
                        position: *position,
                        text: word[a..b].to_string(),
                        position_length: 1,
                    });
                }
            }
            *position += 1;
        };
        for (i, c) in text.char_indices() {
            if c.is_alphanumeric() {
                start.get_or_insert(i);
            } else if let Some(s) = start.take() {
                flush(&mut self.tokens, s, i, &mut position);
            }
        }
        if let Some(s) = start {
            flush(&mut self.tokens, s, text.len(), &mut position);
        }
        IdentifierTokenStream {
            tokens: &mut self.tokens,
            index: 0,
        }
    }
}

impl TokenStream for IdentifierTokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.index += 1;
        self.index <= self.tokens.len()
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        // Los filtros (minúsculas, raíz…) modifican el token en su lugar
        &mut self.tokens[self.index - 1]
    }
}

/// Partes de un identificador (rangos de bytes): `AchOperationCrService` → Ach, Operation, Cr,
/// Service; `XMLParser` → XML, Parser; `apti2Api` → apti, 2, Api. Una palabra simple da una parte.
pub fn split_identifier(word: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = word.char_indices().collect();
    let mut parts = Vec::new();
    let mut start = 0;
    for k in 1..chars.len() {
        let (i, c) = chars[k];
        let prev = chars[k - 1].1;
        let next = chars.get(k + 1).map(|x| x.1);
        let boundary = (prev.is_lowercase() && c.is_uppercase())
            || (prev.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase))
            || (prev.is_alphabetic() && c.is_numeric())
            || (prev.is_numeric() && c.is_alphabetic());
        if boundary {
            parts.push((start, i));
            start = i;
        }
    }
    parts.push((start, word.len()));
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<(String, usize)> {
        let mut t = IdentifierTokenizer::default();
        let mut stream = t.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            out.push((stream.token().text.clone(), stream.token().position));
        }
        out
    }

    #[test]
    fn parte_camel_case_en_la_misma_posicion() {
        assert_eq!(
            tokens("AchOperationCrService llama"),
            vec![
                ("AchOperationCrService".into(), 0),
                ("Ach".into(), 0),
                ("Operation".into(), 0),
                ("Cr".into(), 0),
                ("Service".into(), 0),
                ("llama".into(), 1),
            ]
        );
    }

    #[test]
    fn separa_siglas_numeros_y_respeta_palabras_simples() {
        let words = |w: &str| {
            split_identifier(w)
                .into_iter()
                .map(|(a, b)| w[a..b].to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(words("XMLParser"), vec!["XML", "Parser"]);
        assert_eq!(words("apti2Api"), vec!["apti", "2", "Api"]);
        assert_eq!(words("transferencia"), vec!["transferencia"]);
        // snake_case y kebab-case ya se parten como palabras distintas
        assert_eq!(
            tokens("bs_ach-operations")
                .iter()
                .map(|t| t.0.as_str())
                .collect::<Vec<_>>(),
            vec!["bs", "ach", "operations"]
        );
    }
}
