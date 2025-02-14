use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(AllVariants)]
pub fn derive_all_variants(input: TokenStream) -> TokenStream {
    let syn_item: syn::DeriveInput = syn::parse(input).unwrap();

    let variants = match syn_item.data {
        syn::Data::Enum(enum_item) => enum_item.variants.into_iter().map(|v| v.ident),
        _ => panic!("AllVariants only works on enums"),
    };
    let enum_name = syn_item.ident;
    let len = variants.len();

    let expanded = quote! {
        impl #enum_name {
            pub const ALL_VARIANTS: [Self; #len] = [ #(#enum_name::#variants),* ];
        }
    };

    expanded.into()
}
