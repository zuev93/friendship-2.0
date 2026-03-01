use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr, ItemFn, Stmt};

#[proc_macro_attribute]
pub fn instrumented(attr: TokenStream, item: TokenStream) -> TokenStream {
    let task_id: syn::Path = parse_macro_input!(attr as syn::Path);
    let mut func = parse_macro_input!(item as ItemFn);

    let found = wrap_loop_body(&mut func.block.stmts, &task_id);
    if !found {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "instrumented: no loop found in function body",
        )
        .to_compile_error()
        .into();
    }

    TokenStream::from(quote! { #func })
}

fn wrap_loop_body(stmts: &mut [Stmt], task_id: &syn::Path) -> bool {
    for stmt in stmts.iter_mut() {
        match stmt {
            Stmt::Expr(expr, _) => {
                if wrap_expr(expr, task_id) {
                    return true;
                }
            }
            Stmt::Local(local) => {
                if let Some(init) = &mut local.init {
                    if wrap_expr(&mut init.expr, task_id) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn wrap_expr(expr: &mut Expr, task_id: &syn::Path) -> bool {
    match expr {
        Expr::Loop(loop_expr) => {
            let orig_stmts = &loop_expr.body.stmts;

            let prepend: Stmt = syn::parse_quote! {
                let __inst_start = cortex_m::peripheral::DWT::cycle_count();
            };

            let append_elapsed: Stmt = syn::parse_quote! {
                let __inst_elapsed =
                    cortex_m::peripheral::DWT::cycle_count().wrapping_sub(__inst_start);
            };

            let append_record: Stmt = syn::parse_quote! {
                crate::runtime_stats::record(#task_id, __inst_elapsed);
            };

            let append_heartbeat: Stmt = syn::parse_quote! {
                crate::runtime_stats::heartbeat(#task_id);
            };

            let new_stmts = core::iter::once(prepend)
                .chain(orig_stmts.iter().cloned())
                .chain(core::iter::once(append_elapsed))
                .chain(core::iter::once(append_record))
                .chain(core::iter::once(append_heartbeat))
                .collect();

            loop_expr.body.stmts = new_stmts;
            true
        }
        Expr::Block(block) => wrap_loop_body(&mut block.block.stmts, task_id),
        _ => false,
    }
}
