use quote::ToTokens;
use syn::visit_mut::{self, VisitMut};

/// Inserts observations before statements without changing tail-expression return semantics.
pub fn rust(source: &str) -> Result<String, String> {
    let mut file = syn::parse_file(source).map_err(|e| e.to_string())?;
    struct Observe;
    impl VisitMut for Observe {
        fn visit_block_mut(&mut self, block: &mut syn::Block) {
            visit_mut::visit_block_mut(self, block);
            let mut observed = Vec::new();
            for statement in std::mem::take(&mut block.stmts) {
                use syn::spanned::Spanned;
                let line = statement.span().start().line;
                // Do not insert executable statements among item-only blocks.
                if !matches!(statement, syn::Stmt::Item(_)) {
                    observed.push(syn::parse_quote! { eprintln!("@OJLINE|{}", #line); });
                }
                observed.push(statement);
            }
            block.stmts = observed;
        }
    }
    Observe.visit_file_mut(&mut file);
    Ok(format!(
        "#[allow(unused_macros)] macro_rules! oj_trace {{ ($values:expr) => {{ eprintln!(\"@OJVALUES|{{:?}}\", &$values); }}; }}\n{}",
        file.into_token_stream()
    ))
}

pub const CPP_HOOKS: &str = r#"
#include <unistd.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
static unsigned long oj_count=0;
extern "C" __attribute__((no_instrument_function)) void __cyg_profile_func_enter(void* fn,void*) {
    if (++oj_count>10000) { const char msg[]="@OJLIMIT\n";write(2,msg,sizeof(msg)-1);_exit(120); }
    char buf[96];int n=snprintf(buf,sizeof(buf),"@OJADDR|%lx\n",(unsigned long)(uintptr_t)fn);write(2,buf,n);
}
extern "C" __attribute__((no_instrument_function)) void __cyg_profile_func_exit(void*,void*) {}
"#;

pub const CPP_PRELUDE: &str = r#"
#include <iostream>
#define OJ_TRACE(values) do { std::cerr << "@OJVALUES|["; bool oj_first=true; for(auto const& oj_value : (values)) { if(!oj_first) std::cerr << ','; oj_first=false; std::cerr << oj_value; } std::cerr << "]\n"; } while(0)
#line 1 "solution.cpp"
"#;

pub const PYTHON: &str = include_str!("python_trace.py");

#[cfg(test)]
mod tests {
    #[test]
    fn rust_instrumentation_preserves_tail_expressions_and_rejects_invalid_syntax() {
        let instrumented = super::rust(
            "fn add(a:i32,b:i32)->i32 { let c=a+b; c } fn main(){println!(\"{}\",add(1,2));}",
        )
        .unwrap();
        let parsed = syn::parse_file(&instrumented).unwrap();
        let syn::Item::Fn(function) = &parsed.items[1] else {
            panic!("function expected")
        };
        assert!(matches!(
            function.block.stmts.last(),
            Some(syn::Stmt::Expr(_, None))
        ));
        assert!(super::rust("this is invalid Rust").is_err());
    }
}
