use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed};

parse_attrs_fn!(
	parse_field_attrs -> FieldAttrs {
		flat: bool,
		delegate: bool,
		zlib: bool,
		boxed: bool,
		list: Arg,
	}
);

pub fn derive_readable_impl(input: &DeriveInput) -> TokenStream {
	let type_name = &input.ident;
	let fields = match &input.data {
		Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) => named,
		_ => panic!("only structs with named fields supported"),
	};
	let mut body = quote! {};
	for field in fields {
		let field_ident = field.ident.as_ref().unwrap();
		let FieldAttrs { flat, delegate, zlib, boxed, list } = parse_field_attrs(&field.attrs);
		if flat as u8 + delegate as u8 + zlib as u8 != 1 {
			panic!("{}: field must be one of flat, delegate, or zlib", field_ident);
		}
		if boxed && !flat {
			panic!("{}: boxed field must be flat", field_ident);
		}
		if list.is_some() && zlib {
			panic!("{}: list field cannot be zlib", field_ident);
		}
		if list.is_some() && boxed {
			panic!("{}: list field cannot be boxed", field_ident);
		}
		let initializer = match (flat, boxed, zlib, list) {
			(true, false, false, None) => quote! {
				tr_readable::read_flat(reader, std::ptr::addr_of_mut!((*this).#field_ident))?;
			},
			(true, true, false, None) => quote! {
				tr_readable::read_boxed_flat(reader, std::ptr::addr_of_mut!((*this).#field_ident))?;
			},
			(false, false, false, None) => quote! {
				tr_readable::Readable::read(reader, std::ptr::addr_of_mut!((*this).#field_ident))?;
			},
			(false, false, true, None) => quote! {
				tr_readable::Readable::read(
					&mut tr_readable::zlib(reader)?, std::ptr::addr_of_mut!((*this).#field_ident),
				)?;
			},
			(flat, false, false, Some(list_len_type)) => {
				let array_fn = match flat {
					true => quote! { read_boxed_slice_flat },
					false => quote! { read_boxed_slice_delegate },
				};
				quote! {
					let len = tr_readable::read_flat_get::<_, #list_len_type>(reader)? as usize;
					tr_readable::#array_fn(reader, len, std::ptr::addr_of_mut!((*this).#field_ident))?;
				}
			},
			_ => panic!("invalid attribute combination"),//should be covered by guards
		};
		body = quote! { #body #initializer };
	}
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	quote! {
		impl #impl_generics tr_readable::Readable for #type_name #ty_generics #where_clause {
			unsafe fn read<R: std::io::Read>(reader: &mut R, this: *mut Self) -> std::io::Result<()> {
				#body
				Ok(())
			}
		}
	}
}
