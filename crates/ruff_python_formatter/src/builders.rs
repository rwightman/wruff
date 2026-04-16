use ruff_formatter::{Argument, Arguments, format_args, write};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::context::{NodeLevel, WithNodeLevel};
use crate::other::commas::has_magic_trailing_comma;
use crate::prelude::*;

/// Adds parentheses and indents `content` if it doesn't fit on a line.
pub(crate) fn parenthesize_if_expands<'ast, T>(content: &T) -> ParenthesizeIfExpands<'_, 'ast>
where
    T: Format<PyFormatContext<'ast>>,
{
    ParenthesizeIfExpands {
        inner: Argument::new(content),
        indent: true,
    }
}

pub(crate) struct ParenthesizeIfExpands<'a, 'ast> {
    inner: Argument<'a, PyFormatContext<'ast>>,
    indent: bool,
}

impl ParenthesizeIfExpands<'_, '_> {
    pub(crate) fn with_indent(mut self, indent: bool) -> Self {
        self.indent = indent;
        self
    }
}

impl<'ast> Format<PyFormatContext<'ast>> for ParenthesizeIfExpands<'_, 'ast> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'ast>>) -> FormatResult<()> {
        {
            let mut f = WithNodeLevel::new(NodeLevel::ParenthesizedExpression, f);

            if self.indent {
                let parens_id = f.group_id("indented_parenthesize_if_expands");
                group(&format_args![
                    if_group_breaks(&token("(")),
                    indent_if_group_breaks(
                        &format_args![soft_line_break(), &Arguments::from(&self.inner)],
                        parens_id
                    ),
                    soft_line_break(),
                    if_group_breaks(&token(")"))
                ])
                .with_id(Some(parens_id))
                .fmt(&mut f)
            } else {
                group(&format_args![
                    if_group_breaks(&token("(")),
                    Arguments::from(&self.inner),
                    if_group_breaks(&token(")")),
                ])
                .fmt(&mut f)
            }
        }
    }
}

/// Indents the content by two levels if the enclosing group expands and keeps the closing token on
/// its own line.
pub(crate) fn double_soft_block_indent<'a, Context: 'a>(
    content: &'a impl Format<Context>,
) -> impl Format<Context> + 'a {
    format_with(move |f| {
        write!(
            f,
            [indent(&indent(&format_args![
                soft_line_break(),
                content,
                dedent(&dedent(&soft_line_break())),
            ]))]
        )
    })
}

/// Provides Python specific extensions to [`Formatter`].
pub(crate) trait PyFormatterExtensions<'ast, 'buf> {
    /// A builder that separates each element by a `,` and a [`soft_line_break_or_space`].
    /// It emits a trailing `,` that is only shown if the enclosing group expands. It forces the enclosing
    /// group to expand if the last item has a trailing `comma` and the magical comma option is enabled.
    fn join_comma_separated<'fmt>(
        &'fmt mut self,
        sequence_end: TextSize,
    ) -> JoinCommaSeparatedBuilder<'fmt, 'ast, 'buf>;
}

impl<'buf, 'ast> PyFormatterExtensions<'ast, 'buf> for PyFormatter<'ast, 'buf> {
    fn join_comma_separated<'fmt>(
        &'fmt mut self,
        sequence_end: TextSize,
    ) -> JoinCommaSeparatedBuilder<'fmt, 'ast, 'buf> {
        JoinCommaSeparatedBuilder::new(self, sequence_end)
    }
}

#[derive(Copy, Clone, Debug)]
enum Entries {
    /// No previous entry
    None,
    /// One previous ending at the given position.
    One(TextSize),
    /// More than one entry, the last one ending at the specific position.
    MoreThanOne(TextSize),
}

impl Entries {
    fn position(self) -> Option<TextSize> {
        match self {
            Entries::None => None,
            Entries::One(position) | Entries::MoreThanOne(position) => Some(position),
        }
    }

    const fn is_one_or_more(self) -> bool {
        !matches!(self, Entries::None)
    }

    const fn is_more_than_one(self) -> bool {
        matches!(self, Entries::MoreThanOne(_))
    }

    const fn next(self, end_position: TextSize) -> Self {
        match self {
            Entries::None => Entries::One(end_position),
            Entries::One(_) | Entries::MoreThanOne(_) => Entries::MoreThanOne(end_position),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub(crate) enum TrailingComma {
    /// Add a trailing comma if the group breaks and there's more than one element (or if the last
    /// element has a trailing comma and the magical trailing comma option is enabled).
    #[default]
    MoreThanOne,
    /// Add a trailing comma if the group breaks (or if the last element has a trailing comma and
    /// the magical trailing comma option is enabled).
    OneOrMore,
}

pub(crate) struct JoinCommaSeparatedBuilder<'fmt, 'ast, 'buf> {
    result: FormatResult<()>,
    fmt: &'fmt mut PyFormatter<'ast, 'buf>,
    entries: Entries,
    sequence_start: Option<TextSize>,
    sequence_end: TextSize,
    has_source_line_break: bool,
    trailing_comma: TrailingComma,
}

impl<'fmt, 'ast, 'buf> JoinCommaSeparatedBuilder<'fmt, 'ast, 'buf> {
    fn new(f: &'fmt mut PyFormatter<'ast, 'buf>, sequence_end: TextSize) -> Self {
        Self {
            fmt: f,
            result: Ok(()),
            entries: Entries::None,
            sequence_start: None,
            sequence_end,
            has_source_line_break: false,
            trailing_comma: TrailingComma::default(),
        }
    }

    /// Set the trailing comma behavior for the builder. Trailing commas will only be inserted if
    /// the group breaks, and will _always_ be inserted if the last element has a trailing comma
    /// (and the magical trailing comma option is enabled). However, this setting dictates whether
    /// trailing commas are inserted for single element groups.
    pub(crate) fn with_trailing_comma(mut self, trailing_comma: TrailingComma) -> Self {
        self.trailing_comma = trailing_comma;
        self
    }

    pub(crate) fn with_sequence_start(mut self, sequence_start: TextSize) -> Self {
        self.sequence_start = Some(sequence_start);
        self
    }

    pub(crate) fn entry<T>(
        &mut self,
        node: &T,
        content: &dyn Format<PyFormatContext<'ast>>,
    ) -> &mut Self
    where
        T: Ranged,
    {
        self.entry_with_line_separator(node, content, soft_line_break_or_space())
    }

    pub(crate) fn entry_with_line_separator<N, Separator>(
        &mut self,
        node: &N,
        content: &dyn Format<PyFormatContext<'ast>>,
        separator: Separator,
    ) -> &mut Self
    where
        N: Ranged,
        Separator: Format<PyFormatContext<'ast>>,
    {
        self.result = self.result.and_then(|()| {
            let entry_start = node.start();

            if let Some(previous_end) = self.entries.position() {
                self.has_source_line_break |= self
                    .fmt
                    .context()
                    .source()
                    .contains_line_break(TextRange::new(previous_end, entry_start));
            } else if let Some(sequence_start) = self.sequence_start {
                self.has_source_line_break |= self
                    .fmt
                    .context()
                    .source()
                    .contains_line_break(TextRange::new(sequence_start, entry_start));
            }

            if self.entries.is_one_or_more() {
                write!(self.fmt, [token(","), separator])?;
            }

            self.entries = self.entries.next(node.end());

            content.fmt(self.fmt)
        });

        self
    }

    pub(crate) fn entries<T, I, F>(&mut self, entries: I) -> &mut Self
    where
        T: Ranged,
        F: Format<PyFormatContext<'ast>>,
        I: IntoIterator<Item = (T, F)>,
    {
        for (node, content) in entries {
            self.entry(&node, &content);
        }

        self
    }

    pub(crate) fn nodes<'a, T, I>(&mut self, entries: I) -> &mut Self
    where
        T: Ranged + AsFormat<PyFormatContext<'ast>> + 'a,
        I: IntoIterator<Item = &'a T>,
    {
        for node in entries {
            self.entry(node, &node.format());
        }

        self
    }

    pub(crate) fn finish(&mut self) -> FormatResult<()> {
        self.result.and_then(|()| {
            // Don't add a magic trailing comma when formatting an f-string or t-string expression
            // that always must be flat because the `expand_parent` forces enclosing
            // groups to expand, e.g. `print(f"{(a,)} ")` would format the f-string in
            // flat mode but the `print` call gets expanded because of the `expand_parent`.
            if self
                .fmt
                .context()
                .interpolated_string_state()
                .can_contain_line_breaks()
                == Some(false)
            {
                return Ok(());
            }

            if let Some(last_end) = self.entries.position() {
                let magic_trailing_comma = has_magic_trailing_comma(
                    TextRange::new(last_end, self.sequence_end),
                    self.fmt.context(),
                );
                let preserve_multiline = self.fmt.options().preserve_multiline()
                    && (self.has_source_line_break
                        || self
                            .fmt
                            .context()
                            .source()
                            .contains_line_break(TextRange::new(last_end, self.sequence_end)));

                // If there is a single entry, only keep the magic trailing comma, don't add it if
                // it wasn't there -- unless the trailing comma behavior is set to one-or-more.
                if magic_trailing_comma
                    || self.trailing_comma == TrailingComma::OneOrMore
                    || self.entries.is_more_than_one()
                {
                    if_group_breaks(&token(",")).fmt(self.fmt)?;
                }

                if magic_trailing_comma || preserve_multiline {
                    expand_parent().fmt(self.fmt)?;
                }
            }

            Ok(())
        })
    }
}
