//! CommonMark block layout with native Slint inline styling; no HTML execution.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug)]
pub struct Block {
    pub kind: String,
    pub text: String,
    pub styled: slint::StyledText,
}
fn block(kind: &str, text: String, markdown: &str) -> Block {
    let safe = markdown
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    Block {
        kind: kind.into(),
        styled: slint::StyledText::from_markdown(&safe)
            .unwrap_or_else(|_| slint::StyledText::from_plain_text(&text)),
        text,
    }
}
pub fn parse(source: &str) -> Vec<Block> {
    let mut result = Vec::new();
    let mut text = String::new();
    let mut start = 0;
    let mut kind = "paragraph";
    let mut code = false;
    let mut table = false;
    let mut quote_depth = 0;
    let mut list_depth = 0;
    for (event, range) in Parser::new_ext(
        source,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    )
    .into_offset_iter()
    {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                start = range.start;
                kind = "heading";
                text.clear();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                start = range.start;
                kind = "code";
                code = true;
                text.clear();
            }
            Event::Start(Tag::Table(_)) => {
                start = range.start;
                kind = "code";
                table = true;
                text.clear();
            }
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote_depth -= 1,
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth -= 1,
            Event::Start(Tag::Item) => {
                if !text.is_empty() {
                    result.push(block("paragraph", text.clone(), &text));
                    text.clear();
                }
                start = range.start;
                kind = "list";
                text.push_str("• ");
            }
            Event::Start(Tag::Paragraph) if !code && !table => {
                start = range.start;
                if list_depth == 0 {
                    text.clear();
                    kind = if quote_depth > 0 {
                        "quote"
                    } else {
                        "paragraph"
                    };
                }
            }
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            Event::TaskListMarker(done) => text.push_str(if done { "[x] " } else { "[ ] " }),
            Event::End(TagEnd::TableCell) => text.push_str("  |  "),
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => text.push('\n'),
            Event::End(TagEnd::Table) => {
                result.push(block("code", text.clone(), ""));
                text.clear();
                table = false;
            }
            Event::End(TagEnd::CodeBlock) => {
                result.push(Block {
                    kind: "code".into(),
                    text: text.clone(),
                    styled: slint::StyledText::from_plain_text(&text),
                });
                text.clear();
                code = false;
            }
            Event::End(TagEnd::Heading(_)) => {
                result.push(block("heading", text.clone(), &text));
                text.clear();
            }
            Event::End(TagEnd::Paragraph) if !code && !table => {
                let raw = &source[start..range.end];
                let raw = if kind == "list" {
                    format!("• {raw}")
                } else {
                    raw.into()
                };
                result.push(block(kind, text.clone(), &raw));
                text.clear();
            }
            Event::End(TagEnd::Item) if !text.is_empty() => {
                let raw = source[start..range.end].trim();
                let inline = raw.split_once(' ').map(|(_, rest)| rest).unwrap_or(raw);
                result.push(block("list", text.clone(), &format!("• {inline}")));
                text.clear();
            }
            Event::Rule => result.push(block("divider", String::new(), "")),
            Event::Html(value) | Event::InlineHtml(value) => text.push_str(&value),
            _ => {}
        }
    }
    if !text.is_empty() {
        result.push(block("paragraph", text.clone(), &text));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supports_headings_inline_code_fences_tables_and_links_without_running_html() {
        let blocks = parse(
            "# 标题\n\n**粗体** 和 [动画](oj-animation:abc)\n\n```rust\nfn main() {}\n```\n\n|a|b|\n|-|-|\n|1|2|\n\n- one\n- two\n\n<script>x</script>",
        );
        assert!(
            blocks
                .iter()
                .any(|b| b.kind == "heading" && b.text == "标题")
        );
        assert!(
            blocks
                .iter()
                .any(|b| b.kind == "code" && b.text.contains("fn main"))
        );
        assert!(
            blocks
                .iter()
                .any(|b| b.kind == "code" && b.text.contains("1  |"))
        );
        assert!(blocks.iter().any(|b| b.text.contains("<script>")));
    }
}
