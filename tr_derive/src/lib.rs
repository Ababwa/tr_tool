mod attrs;

use std::borrow::Cow;
use proc_macro2::{TokenStream, Ident, Span};
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed, FieldsUnnamed};
use quote::quote;

parse_attrs_fn!(
	parse_struct_attrs -> StructAttrs {
		skip_after: Arg,
		impl_where: Arg,
	}
);

parse_attrs_fn!(
	parse_field_attrs -> FieldAttrs {
		list: Arg,
		skip: Arg,
		flat: bool,
		zlib: bool,
	}
);

enum InitializerType {
	Named,
	Tuple,
	Unit,
}

fn derive_readable_impl(input: &DeriveInput) -> TokenStream {
	let (fields, initializer_type) = match &input.data {
		Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) => (Some(named), InitializerType::Named),
		Data::Struct(DataStruct { fields: Fields::Unnamed(FieldsUnnamed { unnamed, .. }), .. }) => (Some(unnamed), InitializerType::Tuple),
		Data::Struct(DataStruct { fields: Fields::Unit, .. }) => (None, InitializerType::Unit),
		_ => panic!("only unit structs, tuple structs, or structs with named fields supported"),
	};
	let mut body = quote! {};
	let mut initializer = quote! {};
	let mut tuple_field_num = 0u8..;
	if let Some(fields) = fields {
		for field in fields {
			let FieldAttrs { list, skip, flat, zlib } = parse_field_attrs(&field.attrs);
			let reader = if zlib {
				quote! { &mut tr_readable::get_zlib(reader)? }
			} else {
				quote! { reader }
			};
			let field_ident = match &field.ident {
				Some(field_ident) => Cow::Borrowed(field_ident),
				None => Cow::Owned(Ident::new(&format!("field{}", tuple_field_num.next().unwrap()), Span::call_site())),
			};
			let field_tokens = match (list, flat) {
				(Some(list_type), true) => quote! { read_list_flat::<_, _, #list_type> },
				(Some(list_type), false) => quote! { read_list::<_, _, #list_type> },
				(None, true) => quote! { read_boxed_array_flat },
				(None, false) => quote! { Readable::read },
			};
			let field_tokens = quote! { let #field_ident = tr_readable::#field_tokens(#reader).unwrap(); };
			let field_tokens = match skip {
				Some(skip) => quote! {
					tr_readable::skip(reader, #skip)?;
					#field_tokens
				},
				None => field_tokens,
			};
			body = quote! { #body #field_tokens };
			initializer = quote! { #initializer #field_ident, };
		}
	}
	let StructAttrs { skip_after, impl_where } = parse_struct_attrs(&input.attrs);
	let body = match skip_after {
		Some(skip) => quote! {
			#body
			tr_readable::skip(reader, #skip)?;
		},
		None => body,
	};
	let initializer = match initializer_type {
		InitializerType::Named => quote! { { #initializer } },
		InitializerType::Tuple => quote! { (#initializer) },
		InitializerType::Unit => initializer,
	};
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let where_clause = match (where_clause, impl_where) {
		(Some(where_clause), Some(impl_where)) => quote! { #where_clause, #impl_where },
		(Some(where_clause), None) => quote! { #where_clause },
		(None, Some(impl_where)) => quote! { where #impl_where },
		(None, None) => quote! {},
	};
	let type_name = &input.ident;
	quote! {
		impl #impl_generics tr_readable::Readable for #type_name #ty_generics #where_clause {
			fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
				#body
				Ok(#type_name #initializer)
			}
		}
	}
}


/**
Helper attributes:
* struct
	* `skip_after(num_bytes)`
	* `impl_where(impl_where_clause)`
* field
	* `list(len_type)`
	* `skip(num_bytes)`
	* `flat`
	* `zlib`
*/
#[proc_macro_derive(Readable, attributes(skip_after, impl_where, list, skip, flat, zlib))]
pub fn derive_readable(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	derive_readable_impl(&syn::parse_macro_input!(item)).into()
}
