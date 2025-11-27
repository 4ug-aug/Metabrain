use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid markdown")]
    InvalidMarkdown,
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub frontmatter: Frontmatter,
    pub content: String,
    pub chunks: Vec<String>,
    pub content_hash: String,
}

const CHUNK_SIZE: usize = 500;  // Target chunk size in words
const CHUNK_OVERLAP: usize = 50;  // Overlap between chunks in words

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_file(&self, path: &Path) -> ParseResult<ParsedDocument> {
        let content = fs::read_to_string(path)?;
        self.parse_content(&content)
    }

    pub fn parse_content(&self, content: &str) -> ParseResult<ParsedDocument> {
        let (frontmatter, body) = self.extract_frontmatter(content)?;
        let plain_text = self.markdown_to_plain_text(&body);
        let chunks = self.chunk_text(&plain_text);
        let content_hash = self.compute_hash(content);

        Ok(ParsedDocument {
            frontmatter,
            content: plain_text,
            chunks,
            content_hash,
        })
    }

    fn extract_frontmatter(&self, content: &str) -> ParseResult<(Frontmatter, String)> {
        let content = content.trim();
        
        if !content.starts_with("---") {
            return Ok((Frontmatter::default(), content.to_string()));
        }

        // Find the closing ---
        let rest = &content[3..];
        if let Some(end_idx) = rest.find("\n---") {
            let yaml_content = &rest[..end_idx];
            let body = &rest[end_idx + 4..];
            
            let frontmatter = self.parse_frontmatter_yaml(yaml_content)?;
            return Ok((frontmatter, body.trim().to_string()));
        }

        Ok((Frontmatter::default(), content.to_string()))
    }

    fn parse_frontmatter_yaml(&self, yaml: &str) -> ParseResult<Frontmatter> {
        #[derive(Deserialize)]
        struct RawFrontmatter {
            title: Option<String>,
            #[serde(default)]
            tags: TagsField,
            #[serde(default)]
            aliases: AliasesField,
        }

        #[derive(Deserialize, Default)]
        #[serde(untagged)]
        enum TagsField {
            #[default]
            None,
            Single(String),
            List(Vec<String>),
        }

        #[derive(Deserialize, Default)]
        #[serde(untagged)]
        enum AliasesField {
            #[default]
            None,
            Single(String),
            List(Vec<String>),
        }

        let raw: RawFrontmatter = serde_yaml::from_str(yaml).unwrap_or(RawFrontmatter {
            title: None,
            tags: TagsField::None,
            aliases: AliasesField::None,
        });

        let tags = match raw.tags {
            TagsField::None => vec![],
            TagsField::Single(s) => s.split(',').map(|s| s.trim().to_string()).collect(),
            TagsField::List(v) => v,
        };

        let aliases = match raw.aliases {
            AliasesField::None => vec![],
            AliasesField::Single(s) => vec![s],
            AliasesField::List(v) => v,
        };

        Ok(Frontmatter {
            title: raw.title,
            tags,
            aliases,
        })
    }

    fn markdown_to_plain_text(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut text = String::new();
        let mut in_code_block = false;

        for event in parser {
            match event {
                Event::Text(t) => {
                    if !in_code_block {
                        text.push_str(&t);
                        text.push(' ');
                    }
                }
                Event::Code(c) => {
                    text.push_str(&c);
                    text.push(' ');
                }
                Event::SoftBreak | Event::HardBreak => {
                    text.push(' ');
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    in_code_block = true;
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                }
                Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {
                    text.push('\n');
                }
                Event::Start(Tag::Heading { .. }) | Event::End(TagEnd::Heading(_)) => {
                    text.push('\n');
                }
                _ => {}
            }
        }

        // Clean up extra whitespace
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn chunk_text(&self, text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        
        if words.len() <= CHUNK_SIZE {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < words.len() {
            let end = (start + CHUNK_SIZE).min(words.len());
            let chunk: String = words[start..end].join(" ");
            chunks.push(chunk);

            // Move start forward, accounting for overlap
            start = if end < words.len() {
                end - CHUNK_OVERLAP
            } else {
                end
            };
        }

        chunks
    }

    fn compute_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}

// Add hex encoding dependency alternative
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let parser = MarkdownParser::new();
        let content = r#"---
title: Test Document
tags: [tag1, tag2]
---

# Hello World

This is a test document."#;

        let result = parser.parse_content(content).unwrap();
        assert_eq!(result.frontmatter.title, Some("Test Document".to_string()));
        assert_eq!(result.frontmatter.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_chunking() {
        let parser = MarkdownParser::new();
        let words: Vec<String> = (0..1000).map(|i| format!("word{}", i)).collect();
        let text = words.join(" ");
        
        let chunks = parser.chunk_text(&text);
        assert!(chunks.len() > 1);
    }
}

