use crate::{
    typecheck::{facts_to_query, ValueEq},
    *,
};

pub const RULE_PROOF_KEYWORD: &str = "rule-proof";

#[derive(Clone, Debug)]
pub struct FuncType {
    pub name: Symbol,
    pub input: Vec<ArcSort>,
    pub output: ArcSort,
    pub is_datatype: bool,
    pub has_default: bool,
}



/// Stores resolved typechecking information.
/// TODO make these not public, use accessor methods
#[derive(Clone)]
pub struct TypeInfo {
    // get the sort from the sorts name()
    pub presorts: HashMap<Symbol, PreSort>,
    pub presort_names: HashSet<Symbol>,
    pub sorts: HashMap<Symbol, Arc<dyn Sort>>,
    pub primitives: HashMap<Symbol, Vec<Primitive>>,
    pub func_types: HashMap<Symbol, FuncType>,
    global_types: HashMap<Symbol, ArcSort>,
    pub local_types: HashMap<CommandId, HashMap<Symbol, ArcSort>>,
}

impl Default for TypeInfo {
    fn default() -> Self {
        let mut res = Self {
            presorts: Default::default(),
            presort_names: Default::default(),
            sorts: Default::default(),
            primitives: Default::default(),
            func_types: Default::default(),
            global_types: Default::default(),
            local_types: Default::default(),
        };

        res.add_sort(UnitSort::new(UNIT_SYM.into()));
        res.add_sort(StringSort::new("String".into()));
        res.add_sort(BoolSort::new("bool".into()));
        res.add_sort(I64Sort::new("i64".into()));
        res.add_sort(F64Sort::new("f64".into()));
        res.add_sort(RationalSort::new("Rational".into()));

        res.presort_names.extend(MapSort::presort_names());
        res.presort_names.extend(SetSort::presort_names());
        res.presort_names.extend(VecSort::presort_names());

        res.presorts.insert("Map".into(), MapSort::make_sort);
        res.presorts.insert("Set".into(), SetSort::make_sort);
        res.presorts.insert("Vec".into(), VecSort::make_sort);

        res.add_primitive(ValueEq {
            unit: res.get_sort_nofail(),
        });

        res
    }
}

pub const UNIT_SYM: &str = "Unit";

impl TypeInfo {
    pub(crate) fn infer_literal(&self, lit: &Literal) -> ArcSort {
        match lit {
            Literal::Int(_) => self.sorts.get(&Symbol::from("i64")),
            Literal::F64(_) => self.sorts.get(&Symbol::from("f64")),
            Literal::String(_) => self.sorts.get(&Symbol::from("String")),
            Literal::Bool(_) => self.sorts.get(&Symbol::from("bool")),
            Literal::Unit => self.sorts.get(&Symbol::from("Unit")),
        }
        .unwrap()
        .clone()
    }

    pub fn add_sort<S: Sort + 'static>(&mut self, sort: S) {
        self.add_arcsort(Arc::new(sort)).unwrap()
    }

    pub fn add_arcsort(&mut self, sort: ArcSort) -> Result<(), TypeError> {
        let name = sort.name();

        match self.sorts.entry(name) {
            Entry::Occupied(_) => Err(TypeError::SortAlreadyBound(name)),
            Entry::Vacant(e) => {
                e.insert(sort.clone());
                sort.register_primitives(self);
                Ok(())
            }
        }
    }

    pub fn get_sort_by<S: Sort + Send + Sync>(
        &self,
        pred: impl Fn(&Arc<S>) -> bool,
    ) -> Option<Arc<S>> {
        for sort in self.sorts.values() {
            let sort = sort.clone().as_arc_any();
            if let Ok(sort) = Arc::downcast(sort) {
                if pred(&sort) {
                    return Some(sort);
                }
            }
        }
        None
    }

    pub fn get_sort_nofail<S: Sort + Send + Sync>(&self) -> Arc<S> {
        match self.get_sort_by(|_| true) {
            Some(sort) => sort,
            None => panic!("Failed to lookup sort: {}", std::any::type_name::<S>()),
        }
    }

    pub fn add_primitive(&mut self, prim: impl Into<Primitive>) {
        let prim = prim.into();
        self.primitives.entry(prim.name()).or_default().push(prim);
    }

    pub(crate) fn typecheck_program(
        &mut self,
        program: &Vec<NormCommand>,
    ) -> Result<(), TypeError> {
        for command in program {
            self.typecheck_command(command)?;
        }

        Ok(())
    }

    pub(crate) fn function_to_functype(
        &self,
        func: &NormFunctionDecl,
    ) -> Result<FuncType, TypeError> {
        let input = func
            .schema
            .input
            .iter()
            .map(|name| {
                if let Some(sort) = self.sorts.get(name) {
                    Ok(sort.clone())
                } else {
                    Err(TypeError::Unbound(*name))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let output = if let Some(sort) = self.sorts.get(&func.schema.output) {
            Ok(sort.clone())
        } else {
            Err(TypeError::Unbound(func.schema.output))
        }?;

        Ok(FuncType {
            name: func.name,
            input,
            output: output.clone(),
            is_datatype: output.is_eq_sort() && func.merge.is_none() && func.default.is_none(),
            has_default: func.default.is_some(),
        })
    }

    fn typecheck_ncommand(&mut self, command: &NCommand, id: CommandId) -> Result<(), TypeError> {
        todo!("type check should yield a type-annotated AST")
        // match command {
        //     NCommand::Function(fdecl) => {
        //         if self.sorts.contains_key(&fdecl.name) {
        //             return Err(TypeError::SortAlreadyBound(fdecl.name));
        //         }
        //         if self.is_primitive(fdecl.name) {
        //             return Err(TypeError::PrimitiveAlreadyBound(fdecl.name));
        //         }
        //         let ftype = self.function_to_functype(fdecl)?;
        //         if self.func_types.insert(fdecl.name, ftype).is_some() {
        //             return Err(TypeError::FunctionAlreadyBound(fdecl.name));
        //         }
        //     }
        //     NCommand::NormRule {
        //         rule,
        //         ruleset: _,
        //         name: _,
        //     } => {
        //         self.typecheck_rule(id, rule)?;
        //     }
        //     NCommand::Sort(sort, presort_and_args) => {
        //         self.declare_sort(*sort, presort_and_args)?;
        //     }
        //     NCommand::NormAction(action) => {
        //         self.typecheck_action(id, action, true)?;
        //     }
        //     NCommand::Check(facts) => {
        //         self.typecheck_facts(id, facts)?;
        //         self.verify_normal_form_facts(facts);
        //     }
        //     NCommand::Fail(cmd) => {
        //         self.typecheck_ncommand(cmd, id)?;
        //     }
        //     NCommand::RunSchedule(schedule) => {
        //         self.typecheck_schedule(schedule)?;
        //     }

        //     // TODO cover all cases in typechecking
        //     _ => (),
        // }
        // Ok(())
    }

    fn typecheck_schedule(&mut self, schedule: &NormSchedule) -> Result<(), TypeError> {
        match schedule {
            NormSchedule::Repeat(_times, schedule) => {
                self.typecheck_schedule(schedule)?;
            }
            NormSchedule::Sequence(schedules) => {
                for schedule in schedules {
                    self.typecheck_schedule(schedule)?;
                }
            }
            NormSchedule::Saturate(schedule) => {
                self.typecheck_schedule(schedule)?;
            }
            NormSchedule::Run(run_config) => {
                if let Some(facts) = &run_config.until {
                    assert!(self
                        .local_types
                        .insert(run_config.ctx, Default::default())
                        .is_none());
                    self.typecheck_facts(run_config.ctx, facts)?;
                    self.verify_normal_form_facts(facts);
                }
            }
        }

        Result::Ok(())
    }

    pub(crate) fn typecheck_command(&mut self, command: &NormCommand) -> Result<(), TypeError> {
        assert!(self
            .local_types
            .insert(command.metadata.id, Default::default())
            .is_none());
        self.typecheck_ncommand(&command.command, command.metadata.id)
    }

    pub fn declare_sort(
        &mut self,
        name: impl Into<Symbol>,
        presort_and_args: &Option<(Symbol, Vec<Expr>)>,
    ) -> Result<(), TypeError> {
        let name = name.into();
        if self.func_types.contains_key(&name) {
            return Err(TypeError::FunctionAlreadyBound(name));
        }

        let sort = match presort_and_args {
            Some((presort, args)) => {
                let mksort = self
                    .presorts
                    .get(presort)
                    .ok_or(TypeError::PresortNotFound(*presort))?;
                mksort(self, name, args)?
            }
            None => Arc::new(EqSort { name }),
        };
        self.add_arcsort(sort)
    }

    fn typecheck_rule(&mut self, ctx: CommandId, rule: &Rule) -> Result<(), TypeError> {
        // also check the validity of the ssa
        self.typecheck_facts(ctx, &rule.body)?;
        self.typecheck_actions(ctx, &rule.head)?;
        todo!("do we need to verify normal form?");
        // let mut bindings = self.verify_normal_form_facts(&rule.body);
        // self.verify_normal_form_actions(&rule.head, &mut bindings);
        Ok(())
    }

    fn typecheck_facts(&mut self, ctx: CommandId, facts: &Vec<Fact>) -> Result<(), TypeError> {
        // ROUND TRIP TO CORE RULE AND BACK
        // TODO: in long term, we don't want this round trip to CoreRule query and back just for the type information.
        let query = facts_to_query(facts, self);
        let constraints = query.get_constraints(self)?;
        let problem = Problem { constraints };
        let range = query.atom_terms();
        let assignment = problem
            .solve(range.iter(), |sort: &ArcSort| sort.name())
            .map_err(|e| e.to_type_error())?;

        for (at, ty) in assignment.0.iter() {
            match at {
                AtomTerm::Var(v) => {
                    self.introduce_binding(ctx, *v, ty.clone(), false)?;
                }
                // All the globals should have been introduced
                AtomTerm::Global(_) => {}
                // No need to bind literals as well
                AtomTerm::Literal(_) => {}
            }
        }
        Ok(())
    }

    fn typecheck_actions(
        &mut self,
        ctx: CommandId,
        actions: &Vec<Action>,
    ) -> Result<(), TypeError> {
        for action in actions {
            self.typecheck_action(ctx, action, false)?;
        }
        Ok(())
    }

    fn verify_normal_form_facts(&self, facts: &Vec<Fact>) -> HashSet<Symbol> {
        todo!("should we delete this function?")
        // let mut let_bound: HashSet<Symbol> = Default::default();

        // for fact in facts {
        //     match fact {
        //         NormFact::Compute(var, NormExpr::Call(_head, body)) => {
        //             assert!(!self.global_types.contains_key(var));
        //             assert!(let_bound.insert(*var));
        //             body.iter().for_each(|bvar| {
        //                 if !self.global_types.contains_key(bvar) {
        //                     assert!(let_bound.contains(bvar));
        //                 }
        //             });
        //         }
        //         NormFact::Assign(var, NormExpr::Call(_head, body)) => {
        //             assert!(!self.global_types.contains_key(var));
        //             assert!(let_bound.insert(*var));
        //             body.iter().for_each(|bvar| {
        //                 assert!(!self.global_types.contains_key(bvar));
        //                 assert!(let_bound.insert(*bvar), "Expected {} to be bound", bvar);
        //             });
        //         }
        //         NormFact::AssignVar(lhs, _rhs) => {
        //             assert!(!self.global_types.contains_key(lhs));
        //             assert!(let_bound.insert(*lhs));
        //         }
        //         NormFact::AssignLit(var, _lit) => {
        //             assert!(let_bound.insert(*var));
        //         }
        //         NormFact::ConstrainEq(var1, var2) => {
        //             if !let_bound.contains(var1)
        //                 && !let_bound.contains(var2)
        //                 && !self.global_types.contains_key(var1)
        //                 && !self.global_types.contains_key(var2)
        //             {
        //                 panic!("ConstrainEq on unbound variables");
        //             }
        //         }
        //     }
        // }
        // let_bound
    }

    fn verify_normal_form_actions(
        &self,
        actions: &Vec<NormAction>,
        let_bound: &mut HashSet<Symbol>,
    ) {
        let assert_bound = |var, let_bound: &HashSet<Symbol>| {
            assert!(
                let_bound.contains(var)
                    || self.global_types.contains_key(var)
                    || self.reserved_type(*var).is_some(),
                "Expected {var} to be let bound in body of rule",
            )
        };

        for action in actions {
            match action {
                NormAction::Let(var, NormExpr::Call(_head, body)) => {
                    assert!(let_bound.insert(*var));
                    body.iter().for_each(|bvar| {
                        assert_bound(bvar, let_bound);
                    });
                }
                NormAction::LetVar(v1, v2) => {
                    assert_bound(v2, let_bound);
                    assert!(let_bound.insert(*v1));
                }
                NormAction::LetLit(v1, _lit) => {
                    assert!(let_bound.insert(*v1));
                }
                NormAction::Delete(NormExpr::Call(_head, body)) => {
                    body.iter().for_each(|bvar| {
                        assert_bound(bvar, let_bound);
                    });
                }
                NormAction::Set(NormExpr::Call(_head, body), var) => {
                    body.iter().for_each(|bvar| {
                        assert_bound(bvar, let_bound);
                    });
                    assert_bound(var, let_bound);
                }
                NormAction::Extract(var, variants) => {
                    assert_bound(var, let_bound);
                    assert_bound(variants, let_bound);
                }
                NormAction::Union(v1, v2) => {
                    assert_bound(v1, let_bound);
                    assert_bound(v2, let_bound);
                }
                NormAction::Panic(..) => (),
            }
        }
    }

    fn introduce_binding(
        &mut self,
        ctx: CommandId,
        var: Symbol,
        sort: Arc<dyn Sort>,
        is_global: bool,
    ) -> Result<(), TypeError> {
        if is_global {
            if let Some(_existing) = self.global_types.insert(var, sort) {
                return Err(TypeError::GlobalAlreadyBound(var));
            }
        } else if let Some(existing) = self
            .local_types
            .get_mut(&ctx)
            .unwrap()
            .insert(var, sort.clone())
        {
            return Err(TypeError::LocalAlreadyBound(var, existing, sort));
        }

        Ok(())
    }

    fn typecheck_action(
        &mut self,
        ctx: CommandId,
        action: &Action,
        is_global: bool,
    ) -> Result<(), TypeError> {
        todo!("type check actions should use the constraint-based type checker and yield a type-annotated AST")
        // match action {
        //     NormAction::Let(var, expr) => {
        //         let expr_type = self.typecheck_expr(ctx, expr, true)?.output;

        //         self.introduce_binding(ctx, *var, expr_type, is_global)?;
        //     }
        //     NormAction::LetLit(var, lit) => {
        //         let lit_type = self.infer_literal(lit);
        //         self.introduce_binding(ctx, *var, lit_type, is_global)?;
        //     }
        //     NormAction::Delete(expr) => {
        //         self.typecheck_expr(ctx, expr, true)?;
        //     }
        //     NormAction::Set(expr, other) => {
        //         let func_type = self.typecheck_expr(ctx, expr, true)?;
        //         let other_type = self.lookup(ctx, *other)?;
        //         if func_type.output.name() != other_type.name() {
        //             return Err(TypeError::TypeMismatch(func_type.output, other_type));
        //         }
        //         if func_type.is_datatype {
        //             return Err(TypeError::SetDatatype(func_type));
        //         }
        //     }
        //     NormAction::Union(var1, var2) => {
        //         let var1_type = self.lookup(ctx, *var1)?;
        //         let var2_type = self.lookup(ctx, *var2)?;
        //         if var1_type.name() != var2_type.name() {
        //             return Err(TypeError::TypeMismatch(var1_type, var2_type));
        //         }
        //     }
        //     NormAction::Extract(_var, _variants) => {}
        //     NormAction::LetVar(var1, var2) => {
        //         let var2_type = self.lookup(ctx, *var2)?;
        //         self.introduce_binding(ctx, *var1, var2_type, is_global)?;
        //     }
        //     NormAction::Panic(..) => (),
        // }
        // Ok(())
    }

    pub fn reserved_type(&self, sym: Symbol) -> Option<ArcSort> {
        if sym == RULE_PROOF_KEYWORD.into() {
            Some(self.sorts.get::<Symbol>(&"Proof__".into()).unwrap().clone())
        } else {
            None
        }
    }

    pub fn lookup_global(&self, sym: &Symbol) -> Option<ArcSort> {
        self.global_types.get(sym).cloned()
    }

    pub fn lookup(&self, ctx: CommandId, sym: Symbol) -> Result<ArcSort, TypeError> {
        // special logic for reserved keywords
        if let Some(t) = self.reserved_type(sym) {
            return Ok(t);
        }

        self.global_types
            .get(&sym)
            .map(|x| Ok(x.clone()))
            .unwrap_or_else(|| {
                if let Some(found) = self.local_types.get(&ctx).unwrap().get(&sym) {
                    Ok(found.clone())
                } else {
                    Err(TypeError::Unbound(sym))
                }
            })
    }

    fn set_local_type(
        &mut self,
        ctx: CommandId,
        sym: Symbol,
        sym_type: ArcSort,
    ) -> Result<(), TypeError> {
        if let Some(existing) = self
            .local_types
            .get_mut(&ctx)
            .unwrap()
            .insert(sym, sym_type.clone())
        {
            if existing.name() != sym_type.name() {
                return Err(TypeError::LocalAlreadyBound(sym, existing, sym_type));
            }
        }
        Ok(())
    }

    pub(crate) fn is_primitive(&self, sym: Symbol) -> bool {
        self.primitives.contains_key(&sym) || self.presort_names.contains(&sym)
    }

    /// Lookup a primitive that matches the input types.
    /// Returns the primitive and output type.
    pub(crate) fn lookup_prim(
        &self,
        sym: Symbol,
        input_types: Vec<ArcSort>,
    ) -> Result<(Primitive, ArcSort), TypeError> {
        if let Some(prims) = self.primitives.get(&sym) {
            for prim in prims {
                if let Some(return_type) = prim.accept(&input_types) {
                    return Ok((prim.clone(), return_type));
                }
            }
        }
        Err(TypeError::NoMatchingPrimitive {
            op: sym,
            inputs: input_types.iter().map(|s| s.name()).collect(),
        })
    }

    pub(crate) fn lookup_user_func(&self, sym: Symbol) -> Option<FuncType> {
        self.func_types.get(&sym).cloned()
    }

    pub(crate) fn lookup_func(
        &self,
        sym: Symbol,
        input_types: Vec<ArcSort>,
    ) -> Result<FuncType, TypeError> {
        if let Some(found) = self.func_types.get(&sym) {
            Ok(found.clone())
        } else {
            if let Ok((_prim, output)) = self.lookup_prim(sym, input_types.clone()) {
                return Ok(FuncType {
                    name: sym,
                    input: input_types,
                    output,
                    is_datatype: false,
                    has_default: true,
                });
            }

            Err(TypeError::NoMatchingPrimitive {
                op: sym,
                inputs: input_types.iter().map(|s| s.name()).collect(),
            })
        }
    }

    pub(crate) fn lookup_expr(
        &self,
        ctx: CommandId,
        expr: &NormExpr,
    ) -> Result<FuncType, TypeError> {
        let NormExpr::Call(head, body) = expr;
        let child_types = body
            .iter()
            .map(|var| self.lookup(ctx, *var))
            .collect::<Result<Vec<_>, _>>()?;
        self.lookup_func(*head, child_types)
    }

    pub(crate) fn is_global(&self, sym: Symbol) -> bool {
        self.global_types.contains_key(&sym)
    }

    fn typecheck_expr(
        &mut self,
        ctx: CommandId,
        expr: &NormExpr,
        expect_lookup: bool,
    ) -> Result<FuncType, TypeError> {
        let NormExpr::Call(head, body) = expr;
        let child_types = if let Some(found) = self.func_types.get(head) {
            found.input.clone()
        } else {
            let types = body
                .iter()
                .map(|var| self.lookup(ctx, *var))
                .collect::<Result<Vec<_>, _>>();
            if let Ok(types) = types {
                types
            } else if expect_lookup {
                // return the error
                types?
            } else {
                return Err(TypeError::UnboundFunction(*head));
            }
        };
        for (child_type, var) in child_types.iter().zip(body.iter()) {
            if expect_lookup {
                self.lookup(ctx, *var)?;
            } else {
                self.set_local_type(ctx, *var, child_type.clone())?;
            }
        }

        self.lookup_func(*head, child_types)
    }
}

#[derive(Debug, Clone, Error)]
pub enum TypeError {
    #[error("Arity mismatch, expected {expected} args: {expr}")]
    Arity { expr: Expr, expected: usize },
    #[error(
        "Type mismatch: expr = {expr}, expected = {}, actual = {}, reason: {reason}",
        .expected.name(), .actual.name(),
    )]
    Mismatch {
        expr: Expr,
        expected: ArcSort,
        actual: ArcSort,
        reason: String,
    },
    #[error("Tried to unify too many literals: {}", ListDisplay(.0, "\n"))]
    TooManyLiterals(Vec<Literal>),
    #[error("Unbound symbol {0}")]
    Unbound(Symbol),
    #[error("Undefined sort {0}")]
    UndefinedSort(Symbol),
    #[error("Unbound function {0}")]
    UnboundFunction(Symbol),
    #[error("Function already bound {0}")]
    FunctionAlreadyBound(Symbol),
    #[error("Function declarations are not allowed after a push.")]
    FunctionAfterPush(Symbol),
    #[error("Cannot set the datatype {} to a value. Did you mean to use union?", .0.name)]
    SetDatatype(FuncType),
    #[error("Sort declarations are not allowed after a push.")]
    SortAfterPush(Symbol),
    #[error("Global already bound {0}")]
    GlobalAlreadyBound(Symbol),
    #[error("Local already bound {0} with type {}. Got: {}", .1.name(), .2.name())]
    LocalAlreadyBound(Symbol, ArcSort, ArcSort),
    #[error("Sort {0} already declared.")]
    SortAlreadyBound(Symbol),
    #[error("Primitive {0} already declared.")]
    PrimitiveAlreadyBound(Symbol),
    #[error("Type mismatch: expected {}, actual {}", .0.name(), .1.name())]
    TypeMismatch(ArcSort, ArcSort),
    #[error("Presort {0} not found.")]
    PresortNotFound(Symbol),
    #[error("Cannot type a variable as unit: {0}")]
    UnitVar(Symbol),
    #[error("Failed to infer a type for: {0}")]
    InferenceFailure(Expr),
    #[error("No matching primitive for: ({op} {})", ListDisplay(.inputs, " "))]
    NoMatchingPrimitive { op: Symbol, inputs: Vec<Symbol> },
    #[error("Variable {0} was already defined")]
    AlreadyDefined(Symbol),
    #[error("All alternative definitions considered failed\n{}", .0.iter().map(|e| format!("  {e}\n")).collect::<Vec<_>>().join(""))]
    AllAlternativeFailed(Vec<TypeError>),
}

#[cfg(test)]
mod test {
    use crate::{typechecking::TypeError, EGraph, Error};

    #[test]
    fn test_arity_mismatch() {
        let mut egraph = EGraph::default();

        let res = egraph.parse_and_run_program(
            "
            (relation f (i64 i64))
            (rule ((f a b c)) ())
       ",
        );
        assert!(matches!(
            res,
            Err(Error::TypeError(TypeError::Arity { expected: 2, .. }))
        ));
    }
}
