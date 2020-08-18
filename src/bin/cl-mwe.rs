#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
use rustc_driver::{Callbacks, Compilation};
use rustc_hir as hir;
use rustc_hir::intravisit;
use rustc_hir::intravisit::{walk_expr, Visitor};
use rustc_interface::{interface::Compiler, Queries};
use rustc_middle::{
    hir::map::Map,
    ty::{TyCtxt, TyKind, TypeckResults},
};
use std::env;

struct ClCallback {}
struct ClVisitor<'tcx> {
    hir: Map<'tcx>,
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> Visitor<'tcx> for ClVisitor<'tcx> {
    type Map = Map<'tcx>;

    fn nested_visit_map(&mut self) -> intravisit::NestedVisitorMap<Self::Map> {
        intravisit::NestedVisitorMap::OnlyBodies(self.hir)
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        match expr.kind {
            hir::ExprKind::Closure(_capture_by, _fn_decl, _body_id, span, _option_movability) => {
                println!("Found closure at '{:?}'", span);

                // This works:
                let typeck_results: &TypeckResults<'_> =
                    self.tcx.typeck(self.hir.local_def_id(expr.hir_id));
                let fn_sigs = typeck_results.liberated_fn_sigs();
                let sig = *(fn_sigs.get(expr.hir_id).unwrap());
                println!("Closure signature: {:?}", sig);

                // This fails:
                let ty = self
                    .tcx
                    .type_of(self.hir.local_def_id(expr.hir_id).to_def_id());
                match ty.kind {
                    TyKind::Closure(_, substs) => {
                        println!("Closure signature: {:?}", substs.as_closure().sig())
                    }
                    _ => panic!("Closure has non-closure type"),
                }
            }
            _ => {}
        };
        walk_expr(self, expr)
    }
}

impl Callbacks for ClCallback {
    fn after_analysis<'tcx>(
        &mut self,
        compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        compiler.session().abort_if_errors();
        queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
            let mut visitor: ClVisitor = ClVisitor {
                hir: tcx.hir(),
                tcx: tcx,
            };
            tcx.hir()
                .krate()
                .visit_all_item_likes(&mut visitor.as_deep_visitor());
        });

        compiler.session().abort_if_errors();
        Compilation::Stop
    }
}

fn main() {
    rustc_driver::install_ice_hook();
    rustc_driver::init_rustc_env_logger();

    let args: Vec<String> = env::args().collect();
    let mut cb = ClCallback {};

    std::process::exit(rustc_driver::catch_with_exit_code(move || {
        rustc_driver::run_compiler(&args, &mut cb, None, None)
    }));
}
