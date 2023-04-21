use proc_macro2::TokenStream;
use std::collections::hash_map::HashMap;
use syn::{
    braced, bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, spanned::Spanned,
    Token,
};

struct VarDecl {
    var_ident: syn::Ident,
    colon: Token![:],
    ty: syn::Ident,
}

impl Parse for VarDecl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let var_ident: syn::Ident = input.parse()?;
        let colon = input.parse()?;
        let ty = input.parse()?;

        let var_ident_str = var_ident.to_string();
        if var_ident_str.len() > 1 {
            return Err(syn::Error::new(
                var_ident.span(),
                "Identifier must be a single letter",
            ));
        }

        Ok(Self {
            var_ident,
            colon,
            ty,
        })
    }
}

struct DecodingStatement {
    var_decls: Punctuated<VarDecl, Token![,]>,
    bit_pattern: syn::LitStr,
    body: TokenStream,
}

impl Parse for DecodingStatement {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let var_decls = content.parse_terminated(VarDecl::parse, Token![,])?;

        let mut known_vars = vec![];
        for var_decl in &var_decls {
            if known_vars
                .iter()
                .find(|v| **v == var_decl.var_ident)
                .is_some()
            {
                return Err(syn::Error::new(
                    var_decl.var_ident.span(),
                    "Variable redefined",
                ));
            }
            known_vars.push(var_decl.var_ident.clone());
        }

        let bit_pattern: syn::LitStr = input.parse()?;
        let _arrow: Token![=>] = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let body: TokenStream = content.parse()?;

        Ok(Self {
            var_decls,
            bit_pattern,
            body,
        })
    }
}

struct Field {
    msb: usize,
    lsb: usize,
    ty: syn::Ident,
}

impl Field {
    fn value_mask(&self) -> usize {
        return 1 << (self.msb - self.lsb + 1);
    }
}

impl DecodingStatement {
    fn generate_fields_from_pattern(
        bit_pattern: &syn::LitStr,
        var_decl: &VarDecl,
        type_decls: &Declarations,
    ) -> syn::Result<Field> {
        let bit_pattern_str = bit_pattern.value();
        let expected_char = var_decl.var_ident.to_string().chars().next().unwrap();

        let msb = bit_pattern_str
            .chars()
            .into_iter()
            .enumerate()
            .find(|(_, c)| *c == expected_char)
            .map(|(idx, _)| bit_pattern_str.len() - idx - 1);

        if msb.is_none() {
            let message = format!("Bit pattern does not use variable {}", expected_char);
            return Err(syn::Error::new(bit_pattern.span(), message));
        }

        let lsb = bit_pattern_str
            .chars()
            .rev()
            .into_iter()
            .enumerate()
            .find(|(_, c)| *c == expected_char)
            .map(|(idx, _)| idx);

        let ty = type_decls.find_type(&var_decl.ty);
        if ty.is_none() {
            let message = format!("Unknown type {:?}", var_decl.ty);
            return Err(syn::Error::new(var_decl.ty.span(), message));
        }
        let ty = ty.unwrap();

        let field = Field {
            msb: msb.unwrap(),
            lsb: lsb.unwrap(),
            ty: var_decl.ty.clone(),
        };

        for decl_val in &ty.values {
            let parsed_val: usize = decl_val.value.base10_parse()?;
            if (parsed_val & field.value_mask()) != 0 {
                let message = format!(
                    "Cannot fit value {} of variable {}",
                    parsed_val, var_decl.var_ident
                );
                return Err(syn::Error::new(bit_pattern.span(), message));
            }
        }

        Ok(field)
    }

    fn generate_map_entries(&self, types: &Declarations) -> HashMap<usize, DecoderRow> {
        let mut map = HashMap::new();

        // We need to take the variables and create all combinatorics for each variable.
        for var_decl in &self.var_decls {}

        return map;
    }
}

struct DeclValue {
    label: syn::Ident,
    value: syn::LitInt,
}

impl Parse for DeclValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let label: syn::Ident = input.parse()?;
        let _: Token![=] = input.parse()?;
        let value: syn::LitInt = input.parse()?;
        Ok(Self { label, value })
    }
}

struct Declaration {
    ident: syn::Ident,
    values: Punctuated<DeclValue, Token![,]>,
}

impl Parse for Declaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let content;
        let _ = braced!(content in input);
        let values = content.parse_terminated(DeclValue::parse, Token![,])?;

        let mut known_labels: Vec<String> = vec![];
        let mut known_values: Vec<usize> = vec![];

        for decl_val in values.iter() {
            let label = decl_val.label.to_string();
            if known_labels.iter().find(|v| *v == &label).is_some() {
                return Err(syn::Error::new(decl_val.label.span(), "Duplicated tag"));
            } else {
                known_labels.push(label);
            }

            let value: usize = decl_val.value.base10_parse()?;
            if known_values.iter().find(|v| **v == value).is_some() {
                return Err(syn::Error::new(decl_val.value.span(), "Duplicated value"));
            } else {
                known_values.push(value);
            }
        }

        Ok(Self { ident, values })
    }
}

struct Declarations {
    decls: Punctuated<Declaration, Token![,]>,
}

impl Parse for Declarations {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident != "Declarations" {
            return Err(syn::Error::new(
                ident.span(),
                "Expected `Declarations` identifier",
            ));
        }

        let content;
        let _braces = braced!(content in input);

        let decls = content.parse_terminated(Declaration::parse, Token![,])?;

        let mut known_names: Vec<syn::Ident> = vec![];
        for decl in &decls {
            if known_names
                .iter()
                .find(|n: &&syn::Ident| **n == decl.ident)
                .is_some()
            {
                let err = syn::Error::new(decl.ident.span(), "Duplicated declaration");
                return Err(err);
            }
            known_names.push(decl.ident.clone());
        }

        Ok(Self { decls })
    }
}

impl Declarations {
    fn find_type(&self, ty: &syn::Ident) -> Option<&Declaration> {
        return self
            .decls
            .iter()
            .find(|d| d.ident.to_string() == ty.to_string());
    }
}

struct DecoderRow {
    token_stream: TokenStream,
}

struct DecoderTable {
    decls: Declarations,
    table_ident: syn::Ident,
    element_type: syn::Type,
    table_size: usize,
    statements: Punctuated<DecodingStatement, Token![,]>,
}

impl Parse for DecoderTable {
    /// Parses a SeqItem from a ParseStream
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let decls: Declarations = input.parse()?;

        let table_ident: syn::Ident = input.parse()?;
        let _: Token![:] = input.parse()?;

        let table_args;
        let _brackets = bracketed!(table_args in input);
        let element_type = table_args.parse()?;
        let _: Token![;] = table_args.parse()?;
        let table_size = table_args.parse::<syn::LitInt>()?.base10_parse()?;

        let table_statements;
        let _braces = braced!(table_statements in input);

        let statements = table_statements.parse_terminated(DecodingStatement::parse, Token![,])?;
        Ok(Self {
            decls,
            table_ident,
            element_type,
            table_size,
            statements,
        })
    }
}

impl DecoderTable {
    fn generate(&self) -> TokenStream {
        let enum_decls = self.generate_enum_decls();
        let table_decl = self.generate_table();

        quote::quote! {
            #enum_decls
            #table_decl
        }
        .into()
    }

    fn generate_table(&self) -> TokenStream {
        let table_name = &self.table_ident;
        let element_type = &self.element_type;
        let table_size = &self.table_size;

        // TODO: remove this for loop (test code to trigger an error...)
        for statement in &self.statements {
            for var_decl in &statement.var_decls {
                if let Err(err) = DecodingStatement::generate_fields_from_pattern(
                    &statement.bit_pattern,
                    var_decl,
                    &self.decls,
                ) {
                    return err.to_compile_error();
                }
            }
        }

        // TODO: populate with actual data
        let thingy = quote::quote! {
            OpCode::Ld8RegReg(Register::A, Register::A)
        };

        let thingy = std::iter::repeat(thingy).take(self.table_size);

        quote::quote! {
            pub const #table_name : [#element_type; #table_size] = [
                #(#thingy),*
            ];
        }
    }

    fn generate_enum_decls(&self) -> TokenStream {
        let mut stream = TokenStream::new();
        for enum_decl in &self.decls.decls {
            let name = &enum_decl.ident;
            let val_decls = enum_decl.values.iter().map(|v| {
                let field_name = &v.label;
                let field_value = &v.value;
                quote::quote! {
                    #field_name = #field_value
                }
            });

            let enum_tokens = quote::quote! {
                pub enum #name {
                    #(#val_decls),*
                }
            };
            stream.extend(enum_tokens);
        }
        stream.into()
    }
}

#[proc_macro]
pub fn generate_decoder_table(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let table = parse_macro_input!(input as DecoderTable);
    table.generate().into()
}
