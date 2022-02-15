use miette::{Diagnostic, SourceSpan, SourceCode, SourceOffset};
use pest::{error::LineColLocation, Span};
use thiserror::Error;


#[derive(Debug, Diagnostic, Error)]
pub enum QKaledioscopeError {
    #[error("I/O error reading {subject:?}: {cause}")]
    #[diagnostic(
        help("Double-check the name of the file, and that you have read permissions.")
    )]
    IOError {
        cause: std::io::Error,
        subject: Option<String>
    },

    #[error("Syntax error")]
    #[diagnostic()]
    ParseError {
        description: String,
        #[source_code]
        src: String,
        #[label("{description}")]
        err_span: SourceSpan,
        #[related]
        causes: Vec<QKaledioscopeError>
    }
}
pub type Result<T> = std::result::Result<T, QKaledioscopeError>;

pub(crate) fn wrong_rule_as_parse_error<S>(source: S, description: &str, span: Span, causes: Vec<QKaledioscopeError>) -> QKaledioscopeError
where S: SourceCode + AsRef<str> + ToString
{
    QKaledioscopeError::ParseError {
        description: description.to_string(),
        causes,
        src: source.to_string(),
        err_span: SourceSpan::new(
            SourceOffset::from(span.start()),
            SourceOffset::from(span.end() - span.start())
        )
    }
}

pub(crate) fn rule_error_as_parse_error<S, R>(source: S, error: pest::error::Error<R>) -> QKaledioscopeError
where S: SourceCode + AsRef<str> + ToString,
      R: std::fmt::Debug
{
    let description = match error.variant {
        pest::error::ErrorVariant::ParsingError { negatives, positives } => {
            // TODO: make this prettier
            match (negatives.is_empty(), positives.is_empty()) {
                (false, false) => format!(
                    "unexpected {:?}; expected {:?}",
                    negatives, positives
                ),
                (false, true) => format!("unexpected {:?}", negatives),
                (true, false) => format!("expected {:?}", positives),
                (true, true) => "unknown parsing error".to_owned(),
            }
        },
        pest::error::ErrorVariant::CustomError { message } => message.clone()
    };

    let loc = error.line_col.clone();
    let span = match loc {
        LineColLocation::Pos(pos) =>
            SourceSpan::new(SourceOffset::from_location(&source, pos.0, 1 + pos.1), SourceOffset::from(1)),
        LineColLocation::Span(start, end) => {
            let start = SourceOffset::from_location(&source, start.0, start.1);
            let end = SourceOffset::from_location(&source, end.0, end.1);
            SourceSpan::new(start, SourceOffset::from(end.offset() - start.offset()))
        }
    };
    let err = QKaledioscopeError::ParseError {
        causes: vec![],
        description,
        src: source.to_string(),
        err_span: span
    };
    err
}
