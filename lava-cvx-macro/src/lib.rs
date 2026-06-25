use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

// The single source of truth for special overrides
const OVERRIDES: &[(&str, u16)] = &[
    ("AB3", 1000), // Example alphanumeric code mapped to a unique ID
];

fn parse_cvx_impl(s: &str) -> u16 {
    // 1. Check overrides first
    for &(code, id) in OVERRIDES {
        if code == s {
            return id;
        }
    }
    // 2. Fallback to numeric parsing
    s.parse::<u16>().unwrap_or_else(|_| {
        panic!(
            "Invalid CVX code '{}': must be numeric or in the overrides registry",
            s
        );
    })
}

#[proc_macro]
pub fn cvx(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let val = parse_cvx_impl(&lit.value());
    let expanded = quote! { #val };
    TokenStream::from(expanded)
}

#[proc_macro]
pub fn generate_cvx_registry(_input: TokenStream) -> TokenStream {
    let override_keys: Vec<&str> = OVERRIDES.iter().map(|(k, _)| *k).collect();
    let override_vals: Vec<u16> = OVERRIDES.iter().map(|(_, v)| *v).collect();

    let expanded = quote! {
        pub fn cvx_to_id(code: &str) -> Option<u16> {
            // 1. Check overrides
            match code {
                #( #override_keys => Some(#override_vals), )*
                // 2. Dynamic numeric fallback for unknown/new numeric codes
                _ => code.parse::<u16>().ok(),
            }
        }

        pub fn id_to_cvx(id: u16) -> Option<String> {
            // 1. Check overrides
            match id {
                #( #override_vals => Some(#override_keys.to_string()), )*
                // 2. Format numeric IDs as zero-padded string (at least 2 digits)
                _ => {
                    if id < 10 {
                        Some(format!("0{}", id))
                    } else {
                        Some(id.to_string())
                    }
                }
            }
        }
    };
    TokenStream::from(expanded)
}
