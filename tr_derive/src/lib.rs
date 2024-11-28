mod attrs;
mod readable;

/**
Field helper attributes:
* `flat`: Field is initialized byte-for-byte from the reader.
* `delegate`: Field initialization is delegated to its `Readable` implementation.
* `zlib`: Field is read from a zlib-compressed chunk.
	Initialization is delegated to its `Readable` implementation.
* `boxed`: Field is heap-allocated.
* `list(len_type)`: A `len_type` is read, then a heap-allocated array of that length is read.

A field must be one of `flat`, `delegate`, or `zlib`.

A `boxed` field must be `flat`.

A `list` field must be `flat` or `delegate`.

A field may not be both `boxed` and `list`.
*/
#[proc_macro_derive(Readable, attributes(flat, delegate, zlib, boxed, list))]
pub fn derive_readable(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	readable::derive_readable_impl(&syn::parse_macro_input!(item)).into()
}
