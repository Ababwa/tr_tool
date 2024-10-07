use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed};

parse_attrs_fn!(
	parse_field_attrs -> FieldAttrs {
		flat: bool,
		delegate: bool,
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
		let FieldAttrs { flat, delegate, boxed, list } = parse_field_attrs(&field.attrs);
		if flat == delegate {
			panic!("{}: field must be either flat or delegate", field_ident);
		}
		if boxed && list.is_some() {
			panic!("{}: field cannot be both boxed and list", field_ident);
		}
		if delegate && boxed {
			panic!("{}: field cannot be both delegate and boxed", field_ident);
		}
		let initializer = match (flat, boxed, list) {
			(true, false, None) => quote! {
				tr_readable::read_flat(reader, std::ptr::addr_of_mut!((*this).#field_ident))?;
			},
			(true, true, None) => quote! {
				tr_readable::read_boxed_flat(reader, std::ptr::addr_of_mut!((*this).#field_ident))?;
			},
			(false, false, None) => quote! {
				tr_readable::Readable::read(reader, std::ptr::addr_of_mut!((*this).#field_ident))?;
			},
			(flat, false, Some(list_len_type)) => {
				let array_fn = match flat {
					true => quote! { read_boxed_slice_flat },
					false => quote! { read_boxed_slice_delegate },
				};
				quote! {
					let mut len = tr_readable::read_val_flat::<_, #list_len_type>(reader)? as usize;
					tr_readable::#array_fn(reader, std::ptr::addr_of_mut!((*this).#field_ident), len)?;
				}
			},
			_ => unreachable!("case not covered by guards: ({}, {}, {})", flat, boxed, list.is_some()),
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
