//! Builder implementations for contracts
//!
//! This module provides fluent builders for constructing contracts:
//! - TypeContractBuilder: Build TypeContract instances
//! - FieldContractBuilder: Build FieldContract instances
//!
//! Both builders use functional patterns with method chaining.

use im::HashMap;

use super::types::{Constraint, ContextualHint, FieldContract, TypeContract};

/// Builder for constructing TypeContract instances
pub struct TypeContractBuilder {
    name: String,
    description: String,
    constraints: Vec<Constraint>,
    hints: Vec<ContextualHint>,
    examples: Vec<String>,
    fields: HashMap<String, FieldContract>,
}

impl TypeContractBuilder {
    /// Create a new TypeContractBuilder with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            constraints: Vec::new(),
            hints: Vec::new(),
            examples: Vec::new(),
            fields: HashMap::new(),
        }
    }

    /// Set the contract description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a constraint to the contract
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add a hint to the contract
    pub fn hint(mut self, hint: ContextualHint) -> Self {
        self.hints.push(hint);
        self
    }

    /// Add an example value
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Add a field contract
    pub fn field(mut self, name: impl Into<String>, field: FieldContract) -> Self {
        self.fields = self.fields.update(name.into(), field);
        self
    }

    /// Build the TypeContract
    pub fn build(self) -> TypeContract {
        TypeContract {
            name: self.name,
            description: self.description,
            constraints: self.constraints,
            hints: self.hints,
            examples: self.examples,
            fields: self.fields,
        }
    }
}

/// Builder for constructing FieldContract instances
pub struct FieldContractBuilder {
    name: String,
    field_type: String,
    required: bool,
    description: String,
    constraints: Vec<Constraint>,
    default: Option<String>,
    depends_on: Vec<String>,
    examples: Vec<String>,
}

impl FieldContractBuilder {
    /// Create a new FieldContractBuilder with name and field type
    pub fn new(name: impl Into<String>, field_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            required: false,
            description: String::new(),
            constraints: Vec::new(),
            default: None,
            depends_on: Vec::new(),
            examples: Vec::new(),
        }
    }

    /// Mark this field as required
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set the field description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a constraint to the field
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set the default value
    pub fn default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Add a dependency on another field
    pub fn depends_on(mut self, field: impl Into<String>) -> Self {
        self.depends_on.push(field.into());
        self
    }

    /// Add an example value
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Build the FieldContract
    pub fn build(self) -> FieldContract {
        FieldContract {
            name: self.name,
            field_type: self.field_type,
            required: self.required,
            description: self.description,
            constraints: self.constraints,
            default: self.default,
            depends_on: self.depends_on,
            examples: self.examples,
        }
    }
}
