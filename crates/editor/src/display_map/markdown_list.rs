use language::Node;
use std::num::NonZeroU32;

pub(super) fn extra_indent(
    root: Node<'_>,
    line_start: usize,
    line: &str,
    tab_size: NonZeroU32,
) -> Option<u32> {
    let bytes = line.as_bytes();
    let leading_whitespace = bytes
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let content = line.trim_end();
    let last_character = content.char_indices().next_back()?.0;
    let end = line_start + content.len();
    let mut node = root.descendant_for_byte_range(line_start + last_character, end)?;
    let marker = loop {
        if node.kind() == "list_item" {
            let marker = node.child(0)?;
            if marker.start_byte() >= line_start && marker.start_byte() < end {
                break marker;
            }
        }
        node = node.parent()?;
    };
    let marker = marker
        .next_named_sibling()
        .filter(|node| {
            matches!(
                node.kind(),
                "task_list_marker_checked" | "task_list_marker_unchecked"
            )
        })
        .unwrap_or(marker);

    let mut content_start = marker.end_byte().checked_sub(line_start)?;
    while matches!(bytes.get(content_start), Some(b' ' | b'\t')) {
        content_start += 1;
    }

    let columns = |prefix: &str| {
        prefix.bytes().fold(0, |column, byte| {
            if byte == b'\t' {
                column + tab_size.get() - column % tab_size.get()
            } else {
                column + 1
            }
        })
    };
    Some(columns(line.get(..content_start)?) - columns(line.get(..leading_whitespace)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use language::Parser;

    fn indents(text: &str, tab_size: u32) -> Vec<Option<u32>> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_md::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(text, None).unwrap();
        let mut offset = 0;
        text.split_inclusive('\n')
            .map(|line| {
                let indent =
                    extra_indent(tree.root_node(), offset, line, tab_size.try_into().unwrap());
                offset += line.len();
                indent
            })
            .collect()
    }

    #[test]
    fn bullet_and_numbered_lists() {
        assert_eq!(
            indents("- one\n* two\n+ three\n\n1. four\n12. five\n\n3) six\n", 4),
            vec![
                Some(2),
                Some(2),
                Some(2),
                None,
                Some(3),
                Some(4),
                None,
                Some(3)
            ]
        );
    }

    #[test]
    fn nested_lists_and_continuation_paragraphs() {
        assert_eq!(
            indents(
                "- parent\n  - child\n\n    12. grandchild\n\n  continuation\n",
                4
            ),
            vec![Some(2), Some(2), None, Some(4), None, None]
        );
    }

    #[test]
    fn multiple_list_markers_on_one_line() {
        assert_eq!(
            indents("- - nested\n\n- 1. nested\n\n> - - [ ] task\n", 4),
            vec![Some(4), None, Some(5), None, Some(10)]
        );
    }

    #[test]
    fn ordered_markers_do_not_interrupt_paragraphs() {
        assert_eq!(
            indents("paragraph\n12. still paragraph\n\n12. list item\n", 4),
            vec![None, None, None, Some(4)]
        );
    }

    #[test]
    fn task_lists() {
        assert_eq!(
            indents(
                "- [ ] open\n- [x] done\n- [X] done\n- [link](url)\n- [x](url)\n",
                4
            ),
            vec![Some(6), Some(6), Some(6), Some(2), Some(2)]
        );
    }

    #[test]
    fn lists_inside_blockquotes() {
        assert_eq!(
            indents(
                "> - item\n>   - child\n>\n>> 12. numbered\n>> - [ ] task\n",
                4
            ),
            vec![Some(4), Some(6), None, Some(7), Some(9)]
        );
    }

    #[test]
    fn tab_stops_and_extra_spaces() {
        assert_eq!(
            indents("- parent\n\t- child\n-\titem\n-   [ ]  task\n", 4),
            vec![Some(2), Some(2), Some(4), Some(9)]
        );
    }

    #[test]
    fn code_blocks_and_thematic_breaks_are_not_lists() {
        assert_eq!(
            indents(
                "```md\n- code\n```\n\n    - code\n\n---\n***\n+plus\nplain text\n",
                4
            ),
            vec![None; 10]
        );
        assert_eq!(
            indents("- parent\n\n  ```\n  - code\n  ```\n", 4),
            vec![Some(2), None, None, None, None]
        );
    }

    #[test]
    fn unicode_content_does_not_change_prefix_width() {
        assert_eq!(indents("- café\n  - 中文\n", 4), vec![Some(2), Some(2)]);
    }
}
