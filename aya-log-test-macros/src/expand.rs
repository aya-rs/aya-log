use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_str,
    punctuated::Punctuated,
    Error, Expr, LitStr, Result, Token,
};

use aya_log_common::DisplayHint;
use aya_log_parser::{parse, Fragment};

pub(crate) struct LogArgs {
    pub(crate) buf: Expr,
    pub(crate) format_string: LitStr,
    pub(crate) formatting_args: Option<Punctuated<Expr, Token![,]>>,
}

impl Parse for LogArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let buf: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let format_string: LitStr = input.parse()?;
        let formatting_args: Option<Punctuated<Expr, Token![,]>> = if input.is_empty() {
            None
        } else {
            input.parse::<Token![,]>()?;
            Some(Punctuated::parse_terminated(input)?)
        };

        Ok(Self {
            buf,
            format_string,
            formatting_args,
        })
    }
}

fn string_to_expr(s: Cow<str>) -> Result<Expr> {
    parse_str(&format!("\"{}\"", s))
}

fn hint_to_expr(hint: DisplayHint) -> Result<Expr> {
    match hint {
        DisplayHint::Default => parse_str("::aya_log_common::DisplayHint::Default"),
        DisplayHint::LowerHex => parse_str("::aya_log_common::DisplayHint::LowerHex"),
        DisplayHint::UpperHex => parse_str("::aya_log_common::DisplayHint::UpperHex"),
        DisplayHint::IPv4 => parse_str("::aya_log_common::DisplayHint::IPv4"),
        DisplayHint::IPv6 => parse_str("::aya_log_common::DisplayHint::IPv6"),
    }
}

pub(crate) fn log(args: LogArgs) -> Result<TokenStream> {
    let format_string = args.format_string;
    let format_string_val = format_string.value();
    let fragments = parse(&format_string_val)
        .map_err(|_| Error::new(format_string.span(), "failed to parse format string"))?;

    let mut values = Vec::new();
    let mut hints = Vec::new();
    let mut arg_i = 0;
    for fragment in fragments {
        match fragment {
            Fragment::Literal(s) => {
                values.push(string_to_expr(s)?);
                hints.push(hint_to_expr(DisplayHint::Default)?);
            }
            Fragment::Parameter(p) => {
                let arg = match args.formatting_args {
                    Some(ref args) => args[arg_i].clone(),
                    None => return Err(Error::new(format_string.span(), "no arguments provided")),
                };
                values.push(arg);
                hints.push(hint_to_expr(p.hint)?);
                arg_i += 1;
            }
        }
    }
    let num_args = values.len();

    let values_iter = values.iter();
    let hints_iter = hints.iter();

    let buf = args.buf;

    Ok(quote! {
        {
            if let Ok(header_len) = ::aya_log_common::write_record_header(
                &mut #buf,
                "test",
                ::aya_log_common::Level::Info,
                "test",
                "test.rs",
                123,
                #num_args
            ) {
                let mut record_len = header_len;

                use ::aya_log_common::WriteToBuf;
                #(
                    if record_len >= #buf.len() {
                        return ();
                    }
                    record_len += { #values_iter }.write(&mut #buf[record_len..], #hints_iter).unwrap();
                )*
            }
        }
    })
}
