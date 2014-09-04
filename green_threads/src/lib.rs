#![crate_name="green_threads"]
#![crate_type="dylib"]

#![feature(quote, plugin_registrar, macro_rules)]

extern crate syntax;
extern crate rustc;

use std::gc::{Gc, GC};

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{ExtCtxt, MacResult, MacroDef, DummyResult};
use syntax::fold::{mod, Folder};
use syntax::parse;
use syntax::util::small_vector::SmallVector;

use rustc::plugin::Registry;

// Helper struct that allows us to use multiple Items as a MacResult
struct MacItems {
    items: Vec<Gc<ast::Item>>,
}

impl MacItems {
    pub fn new(items: Vec<Gc<ast::Item>>) -> Box<MacResult+'static> {
        box MacItems { items: items } as Box<MacResult+'static>
    }
}

impl MacResult for MacItems {
    fn make_def(&self) -> Option<MacroDef> { None }
    fn make_expr(&self) -> Option<Gc<ast::Expr>> { None }
    fn make_pat(&self) -> Option<Gc<ast::Pat>> { None }
    fn make_stmt(&self) -> Option<Gc<ast::Stmt>> { None }

    fn make_items(&self) -> Option<SmallVector<Gc<ast::Item>>> {
        Some(SmallVector::many(self.items.clone()))
    }
}

#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(registrar: &mut Registry) {
    registrar.register_macro("green", green)
}

fn green(cx: &mut ExtCtxt, sp: codemap::Span, tts: &[ast::TokenTree]) -> Box<MacResult+'static> {
    let i = match parse(cx, tts) {
        Some(i) => i,
        None    => DummyResult::expr(sp),
    };

    i
}

struct GreenFolder {
    stmt: Gc<ast::Stmt>,
}

impl GreenFolder {
    pub fn new(s: Gc<ast::Stmt>) -> GreenFolder {
        GreenFolder {
            stmt: s,
        }
    }

    fn gen_block(&self, old_block: &Gc<ast::Block>) -> Gc<ast::Block> {
        // Prepend a call to our yield macro to the statements in this block.
        let mut new_stmts = old_block.stmts.clone();
        new_stmts.insert(0, self.stmt.clone());

        // This would be nicer if I could figure out how to get the
        // "..*block" syntax to work properly...
        let new_block = box (GC) ast::Block {
            stmts: new_stmts,

            view_items: old_block.view_items.clone(),
            expr: old_block.expr.clone(),
            id: old_block.id,
            rules: old_block.rules.clone(),
            span: old_block.span.clone(),
        };

        new_block
    }
}

impl Folder for GreenFolder {
    fn fold_expr(&mut self, e: Gc<ast::Expr>) -> Gc<ast::Expr> {
        let folded = fold::noop_fold_expr(e, self);

        let new_node = match folded.node {
            ast::ExprForLoop(pat, expr, block, ident) => {
                println!("found for loop");

                ast::ExprForLoop(pat, expr, self.gen_block(&block), ident)
            },
            ast::ExprWhile(expr, block, ident) => {
                println!("found while loop");

                ast::ExprWhile(expr, self.gen_block(&block), ident)
            },
            ast::ExprLoop(block, ident) => {
                println!("found loop");

                ast::ExprLoop(self.gen_block(&block), ident)
            },
            ref n => n.clone(),
        };

        let new_expr = box (GC) ast::Expr {
            id: folded.id,
            node: new_node,
            span: folded.span,
        };

        new_expr
    }

    fn fold_item_underscore(&mut self, i: &ast::Item_) -> ast::Item_ {
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


fn parse(cx: &mut ExtCtxt, tts: &[ast::TokenTree]) -> Option<Box<MacResult+'static>> {
    use syntax::print::pprust;

    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(),
                                                Vec::from_slice(tts));

    let item = match parser.parse_item(vec![]) {
        Some(i) => {
            let items: Vec<Gc<ast::Item>> = cx.expander().
                fold_item(i).
                move_iter().
                collect();

            let mut folded = Vec::new();
            for i in items.move_iter() {
                let new_stmt = quote_stmt!(&cx, green_yield!(););
                let f = GreenFolder::new(new_stmt).fold_item(i).expect_one("GreenFolder returned more than 1 item");
                folded.push(f);
            }

            Some(MacItems::new(folded))
        },
        None => {
            cx.span_err(parser.span, "Expected item");
            None
        },
    };

    item
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
