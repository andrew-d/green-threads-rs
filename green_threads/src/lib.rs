#![crate_name="green_threads"]
#![crate_type="dylib"]

#![feature(quote, phase, plugin_registrar, macro_rules)]

extern crate syntax;
extern crate rustc;

#[phase(plugin, link)]
extern crate log;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{ExtCtxt, Modifier, ItemModifier};
use syntax::fold::{mod, Folder};
use syntax::parse::token::intern;
use syntax::ptr::{P};

use rustc::plugin::Registry;


#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(registrar: &mut Registry) {
    registrar.register_syntax_extension(intern("greenify"), Modifier(box greenify as Box<ItemModifier + 'static>));
}

fn greenify(cx: &mut ExtCtxt, _sp: codemap::Span, _attr: &ast::MetaItem, it: P<ast::Item>) -> P<ast::Item> {
    // This is what we insert to "yield".
    let new_stmt = quote_stmt!(&cx, green_yield!(););

    // Do the actual insertion.
    let f = GreenFolder::new(new_stmt).fold_item(it).expect_one("GreenFolder returned more than 1 item");

    f
}

struct GreenFolder {
    stmt: P<ast::Stmt>,
}

impl GreenFolder {
    pub fn new(s: P<ast::Stmt>) -> GreenFolder {
        GreenFolder {
            stmt: s,
        }
    }

    fn gen_block(&self, old_block: &P<ast::Block>) -> P<ast::Block> {
        // Prepend a call to our yield macro to the statements in this block.
        let mut new_stmts = old_block.stmts.clone();
        new_stmts.insert(0, self.stmt.clone());

        // This would be nicer if I could figure out how to get the
        // "..*block" syntax to work properly...
        let new_block = P(ast::Block {
            stmts: new_stmts,

            view_items: old_block.view_items.clone(),
            expr: old_block.expr.clone(),
            id: old_block.id,
            rules: old_block.rules.clone(),
            span: old_block.span.clone(),
        });

        new_block
    }
}

impl Folder for GreenFolder {
    fn fold_expr(&mut self, e: P<ast::Expr>) -> P<ast::Expr> {
        let folded = e.map(|t| fold::noop_fold_expr(t, self));

        let new_expr = folded.and_then(|folded| {
            let n = match folded.node {
                ast::ExprForLoop(pat, expr, block, ident) => {
                    debug!("found for loop");

                    ast::ExprForLoop(pat, expr, self.gen_block(&block), ident)
                },
                ast::ExprWhile(expr, block, ident) => {
                    debug!("found while loop");

                    ast::ExprWhile(expr, self.gen_block(&block), ident)
                },
                ast::ExprLoop(block, ident) => {
                    debug!("found loop");

                    ast::ExprLoop(self.gen_block(&block), ident)
                },
                ref n => n.clone(),
            };

            P(ast::Expr {
                id:   folded.id,
                node: n,
                span: folded.span,
            })
        });

        new_expr
    }

    fn fold_item_underscore(&mut self, i: ast::Item_) -> ast::Item_ {
        let i = fold::noop_fold_item_underscore(i, self);

        let new_item = match i {
            ast::ItemFn(ref decl, ref style, ref abi, ref generics, ref block) => {
                let new_block = self.gen_block(block);

                ast::ItemFn(decl.clone(), style.clone(),
                            abi.clone(), generics.clone(),
                            new_block
                           )
            },
            _ => i,
        };

        new_item
    }
}


/**
 * This is a simple macro that could "actually" do green-thread yielding.
 */
#[macro_export]
macro_rules! green_yield {
    () => {
        println!("[green_yield] This should actually yield the green thread.");
    };
}
