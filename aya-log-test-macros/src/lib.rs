use proc_macro::TokenStream;
use syn::parse_macro_input;

mod expand;

#[proc_macro]
pub fn log(args: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as expand::LogArgs);
    expand::log(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
