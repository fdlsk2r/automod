#![doc(html_root_url = "https://docs.rs/automod/1.0.14")]
#![allow(clippy::enum_glob_use, clippy::needless_pass_by_value)]

extern crate proc_macro;

mod error;

use error::{Error, Result};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, LitStr};

struct Arg {
    path: LitStr,
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Arg {
            path: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn dir(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Arg);
    let rel_path = input.path.value();

    let dir = match env::var_os("CARGO_MANIFEST_DIR") {
        Some(manifest_dir) => PathBuf::from(manifest_dir).join(rel_path),
        None => PathBuf::from(rel_path),
    };

    let expanded = match source_file_names(dir) {
        Ok(names) => names.into_iter().map(|name| mod_item(name)).collect(),
        Err(err) => syn::Error::new(Span::call_site(), err).to_compile_error(),
    };

    TokenStream::from(expanded)
}

fn mod_item(name: String) -> TokenStream2 {
    let mut module_name = name.replace('-', "_");
    if module_name.starts_with(|ch: char| ch.is_ascii_digit()) {
        module_name.insert(0, '_');
    }

    println!("{}", name);

    let path = Option::into_iter(if name == module_name {
        None
    } else {
        Some(format!("{}.rs", name))
    });

    let ident = Ident::new(&module_name, Span::call_site());

    quote! {
        #(#[path = #path])*
        mod #ident;
        pub use #ident::*;
    }
}

fn source_file_names<P: AsRef<Path>>(dir: P) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut failures = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        if file_name == "mod.rs" || file_name == "lib.rs" || file_name == "lib" {
            continue;
        }

        let path = Path::new(&file_name);
        if path.extension() == Some(OsStr::new("rs")) {
            match file_name.into_string() {
                Ok(mut utf8) => {
                    utf8.truncate(utf8.len() - ".rs".len());
                    names.push(utf8);
                }
                Err(non_utf8) => {
                    failures.push(non_utf8);
                }
            }
        }
    }

    failures.sort();
    if let Some(failure) = failures.into_iter().next() {
        return Err(Error::Utf8(failure));
    }

    if names.is_empty() {
        return Err(Error::Empty);
    }

    names.sort();
    Ok(names)
}
