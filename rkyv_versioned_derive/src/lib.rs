use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DataEnum, DeriveInput, Fields, Generics, Ident, LitStr};

/// Derive macro for automatically implementing VersionedArchiveContainer for an enum.
///
/// Accepts `#[rkyv_versioned(archive_type_name = "...")]` to override the string the type ID
/// is hashed from, which defaults to the container's identifier.
///
/// See the `VersionedContainer` trait and the example in the `rkyv_versioned` crate for more
/// details.
#[proc_macro_derive(VersionedArchiveContainer, attributes(rkyv_versioned))]
pub fn derive_versioned_archive_container(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    let result = match input.data {
        Data::Enum(data_enum) => {
            generate(input.ident, data_enum, input.generics, &input.attrs)
        }
        _ => {
            quote! { compile_error!("#[derive(VersionedArchiveContainer)] is only defined for enums") }
        }
    };

    result.into()
}

/// Reads `#[rkyv_versioned(archive_type_name = "...")]`, if present.
///
/// Unknown keys inside the namespace are rejected rather than ignored, so a typo does not
/// silently leave the default in place.
fn parse_archive_type_name(attrs: &[Attribute]) -> Result<Option<String>, syn::Error> {
    let mut archive_type_name = None;

    for attr in attrs
        .iter()
        .filter(|attr| attr.path().is_ident("rkyv_versioned"))
    {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("archive_type_name") {
                return Err(meta.error(
                    "unsupported rkyv_versioned attribute, expected `archive_type_name = \"...\"`",
                ));
            }

            let name: LitStr = meta.value()?.parse()?;
            if name.value().is_empty() {
                return Err(syn::Error::new(
                    name.span(),
                    "archive_type_name must not be empty",
                ));
            }

            if archive_type_name.replace(name.value()).is_some() {
                return Err(meta.error("archive_type_name specified more than once"));
            }

            Ok(())
        })?;
    }

    Ok(archive_type_name)
}

fn generate(
    enum_name: Ident,
    data_enum: DataEnum,
    generics: Generics,
    attrs: &[Attribute],
) -> TokenStream {
    // The type ID is a hash of this string, so overriding it changes the serialized format
    let string_name = match parse_archive_type_name(attrs) {
        Ok(Some(name)) => name,
        Ok(None) => enum_name.to_string(),
        Err(error) => return error.to_compile_error(),
    };
    let mut error_messages = quote! {};

    // Parse the enum variants
    let mut valid_versions: Vec<TokenStream> = vec![];
    let mut newest_version_id: u32 = 0;
    let mut match_branches = quote! {};
    for (variant_index, variant) in data_enum.variants.iter().enumerate() {
        // Cache this for error messages
        let current_field_debug_name = format!("{}::{}", enum_name, variant.ident);

        // Only unnamed fields are supported
        if let Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() != 1 {
                let error_string = format!("Only one unnamed field per enum variant is supported, found multiple fields in {current_field_debug_name}");
                error_messages.extend(quote! {
                    compile_error!(#error_string);
                });
            } else {
                // TODO: Allow overriding of this with #[rkyv_versioned(version = X)]
                let variant_index_as_u32 = variant_index as u32;
                valid_versions.push(quote! { #variant_index_as_u32 });
                newest_version_id = newest_version_id.max(variant_index_as_u32);

                let branch_name = &variant.ident;
                match_branches.extend(quote! {
                    #enum_name::#branch_name(_) => #variant_index_as_u32,
                });
            }
        } else {
            let error_string = format!(
                "Only unnamed fields supported in enum variants, unsupported variant found in {current_field_debug_name}"
            );
            error_messages.extend(quote! {
                compile_error!(#error_string);
            });
        }
    }

    // We only care about the number of lifetimes since we'll just use anonymous ones
    let lifetime_params = generics
        .lifetimes()
        .map(|_| quote! {'_})
        .collect::<Vec<_>>();
    let lifetime_decl = match lifetime_params.len() {
        0 => quote! {},
        _ => quote! {<#(#lifetime_params),*>},
    };

    quote! {
        #error_messages

        #[automatically_derived]
        // Automatically derived implementation of VersionedContainer for #enum_name
        impl ::rkyv_versioned::VersionedContainer for #enum_name #lifetime_decl {
            const ARCHIVE_TYPE_ID : u32 =
                ::rkyv_versioned::const_crc32::crc32(#string_name.as_bytes());
            const ARCHIVE_TYPE_NAME : &'static str = #string_name;
            const NEWEST_VERSION_ID : u32 = #newest_version_id;

            fn get_entry_version_id(&self) -> u32 {
                match self {
                    #match_branches
                }
            }

            fn is_valid_version_id(version : u32) -> bool {
                match version {
                    #(#valid_versions)|* => true,
                    _ => false,
                }
            }
        }
    }
}
