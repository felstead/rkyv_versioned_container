use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Fields, Generics, Ident};

#[proc_macro_derive(VersionedArchiveContainer)]
pub fn derive_versioned_archive_container(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    let result = match input.data {
        Data::Enum(data_enum) => generate(input.ident, data_enum, input.generics),
        _ => {
            quote! { compile_error!("#[derive(VersionedArchiveContainer)] is only defined for enums") }
        }
    };

    result.into()
}

fn generate(enum_name: Ident, data_enum: DataEnum, generics: Generics) -> TokenStream {
    let string_name = enum_name.to_string();
    let mut error_messages = quote! {};

    // Parse the enum variants
    let mut valid_versions: Vec<TokenStream> = vec![];
    let mut match_branches = quote! {};
    for (variant_index, variant) in data_enum.variants.iter().enumerate() {
        // Cache this for error messages
        let current_field_debug_name = format!("{}::{}", enum_name, variant.ident);

        // Only unnamed fields are supported
        if let Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() != 1 {
                let error_string = format!("Only one unnamed field per enum variant is supported, found multiple fields in {}", current_field_debug_name);
                error_messages.extend(quote! {
                    compile_error!(#error_string);
                });
            } else {
                // TODO: Allow overriding of this with #[rkyv_util_version(X)]
                let variant_index_as_u32 = variant_index as u32;
                valid_versions.push(quote! { #variant_index_as_u32 });

                let branch_name = &variant.ident;
                match_branches.extend(quote! {
                    #enum_name::#branch_name(_) => #variant_index_as_u32,
                });
            }
        } else {
            let error_string = format!(
                "Only unnamed fields supported in enum variants, unsupported variant found in {}",
                current_field_debug_name
            );
            error_messages.extend(quote! {
                compile_error!(#error_string);
            });
        }
    }

    // Build the valid version comparison - there is probably a more concise way to do this
    let valid_version_comparison = quote! {
        #(#valid_versions)|*
    };

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
        impl VersionedContainer for #enum_name #lifetime_decl {
            const ARCHIVE_TYPE_ID : u32 = const_crc32::crc32(#string_name.as_bytes());

            fn get_entry_version_id(&self) -> u32 {
                match self {
                    #match_branches
                }
            }

            fn is_valid_version_id(version : u32) -> bool {
                match version {
                    #valid_version_comparison => true,
                    _ => false,
                }
            }

            fn get_ref_from_tagged_versioned_bytes(buf : & [u8]) -> Result<&Self::Archived, rkyv::rancor::Error> {
                const HEADER_SIZE : usize = core::mem::size_of::<ArchivedTaggedVersionedContainerHeaderOnly>();

                let header = rkyv::access::<ArchivedTaggedVersionedContainerHeaderOnly, rkyv::rancor::Error>(&buf[0..HEADER_SIZE])?;

                // Ensure the type header is correct
                if header.0 != Self::ARCHIVE_TYPE_ID {
                    rkyv::rancor::fail!(UnexpectedTypeError(Self::ARCHIVE_TYPE_ID, header.0.into()));
                }

                // Ensure the version header is valid
                if Self::is_valid_version_id(header.1.into()) {
                    let archived = rkyv::access::<ArchivedTaggedVersionedContainer<Self>, rkyv::rancor::Error>(&buf)?;
                    Ok(&archived.2)
                } else {
                    rkyv::rancor::fail!(UnsupportedVersionError(header.1.into()))
                }
            }

            fn to_tagged_versioned_bytes(item : &Self) -> Result<AlignedVec, rkyv::rancor::Error>
                where Self: for<'b> Serialize<HighSerializer<AlignedVec, ArenaHandle<'b>, rkyv::rancor::Error>>
            {
                let container = TaggedVersionedContainer (Self::ARCHIVE_TYPE_ID, item.get_entry_version_id(), item);
                rkyv::to_bytes(&container)
            }
        }
    }
}
