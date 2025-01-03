pub mod contracts;
pub mod inter_record;
pub mod matcher;
pub mod visitor;

use crate::analysis::unsafety_isolation::{
    hir_visitor::{ContainsUnsafe, RelatedFnCollector},
    UnsafetyIsolationCheck,
};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use visitor::{BodyVisitor, CheckResult};

pub struct SenryxCheck<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub threshhold: usize,
}

impl<'tcx> SenryxCheck<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, threshhold: usize) -> Self {
        Self { tcx, threshhold }
    }

    pub fn start(&self) {
        let related_items = RelatedFnCollector::collect(self.tcx); // find all func
        let hir_map = self.tcx.hir();
        for (_, &ref vec) in &related_items {
            for (body_id, _span) in vec {
                let (function_unsafe, block_unsafe) =
                    ContainsUnsafe::contains_unsafe(self.tcx, *body_id);
                let def_id = hir_map.body_owner_def_id(*body_id).to_def_id();
                if block_unsafe {
                    self.check_soundness(def_id);
                }
                if function_unsafe {
                    self.annotate_safety(def_id);
                }
            }
        }
    }

    pub fn check_soundness(&self, def_id: DefId) {
        let check_results = self.body_visit_and_check(def_id);
        if check_results.len() > 0 {
            Self::show_check_results(def_id, check_results);
        }
    }

    pub fn annotate_safety(&self, def_id: DefId) {
        let check_results = self.body_visit_and_check(def_id);
        if check_results.len() > 0 {
            Self::show_check_results(def_id, check_results);
        }
    }

    pub fn body_visit_and_check(&self, def_id: DefId) -> Vec<CheckResult> {
        let mut uig_checker = UnsafetyIsolationCheck::new(self.tcx);
        let func_type = uig_checker.get_type(def_id);
        let mut body_visitor = BodyVisitor::new(self.tcx, def_id, 0);
        if func_type == 1 {
            let func_cons = uig_checker.search_constructor(def_id);
            for func_con in func_cons {
                let mut cons_body_visitor = BodyVisitor::new(self.tcx, func_con, 0);
                cons_body_visitor.path_forward_check();
                // TODO: cache fields' states

                // TODO: update method body's states

                // analyze body's states
                body_visitor.path_forward_check();
            }
        } else {
            body_visitor.path_forward_check();
        }
        return body_visitor.check_results;
    }

    pub fn show_check_results(def_id: DefId, check_results: Vec<CheckResult>) {
        println!("--------In {:?}---------", def_id);
        for check_result in check_results {
            println!(
                "  Unsafe api {:?}: {} passed, {} failed!",
                check_result.func_name,
                check_result.passed_contracts.len(),
                check_result.failed_contracts.len()
            );
            for failed_contract in check_result.failed_contracts {
                println!("      Contract failed: {:?}", failed_contract);
            }
        }
    }
}
