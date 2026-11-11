//! Finding nodes: by accessible name, role, ancestor and path, in tree order.

use super::{AccessTree, NodeKey};
use crate::runtime::access::Role;
use accesskit::NodeId;
use std::collections::HashSet;

/// How a query matches a node's accessible name.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameMatch {
    /// The whole name.
    Exact(String),
    /// A part of the name.
    Contains(String),
}

impl NameMatch {
    fn matches(&self, name: &str) -> bool {
        match self {
            NameMatch::Exact(wanted) => name == wanted,
            NameMatch::Contains(part) => name.contains(part.as_str()),
        }
    }
}

/// Which nodes [`AccessTree::find`] returns: every condition set must hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    name: Option<NameMatch>,
    role: Option<Role>,
    within: Option<Box<Query>>,
}

impl Query {
    /// Nodes whose accessible name is `name`.
    pub fn named(name: impl Into<String>) -> Query {
        Query { name: Some(NameMatch::Exact(name.into())), ..Query::default() }
    }

    /// Nodes whose accessible name contains `text`.
    pub fn name_contains(text: impl Into<String>) -> Query {
        Query { name: Some(NameMatch::Contains(text.into())), ..Query::default() }
    }

    /// Nodes of `role`.
    pub fn role(role: Role) -> Query {
        Query { role: Some(role), ..Query::default() }
    }

    /// The node named by the last of `names` that lies below a node named by each name before it, in order; the
    /// nodes between them need no name, so unnamed groups do not break a path. `None` for no names.
    pub fn path<S: Into<String>>(names: impl IntoIterator<Item = S>) -> Option<Query> {
        names.into_iter().map(Query::named).reduce(|ancestor, node| node.within(ancestor))
    }

    /// This query, narrowed to nodes of `role`.
    pub fn with_role(mut self, role: Role) -> Query {
        self.role = Some(role);
        self
    }

    /// This query, narrowed to nodes below a node `ancestor` matches.
    pub fn within(mut self, ancestor: Query) -> Query {
        self.within = Some(Box::new(ancestor));
        self
    }
}

impl AccessTree {
    /// The nodes `query` matches, in tree order, among those assistive technology reports.
    pub fn find(&self, query: &Query) -> Vec<NodeKey> {
        self.find_among(self.in_order(), query)
    }

    fn find_among(&self, nodes: Vec<NodeKey>, query: &Query) -> Vec<NodeKey> {
        let ancestors: Option<HashSet<NodeId>> =
            query.within.as_deref().map(|within| self.find_among(nodes.clone(), within).into_iter().map(|key| NodeId(key.0)).collect());
        nodes
            .into_iter()
            .filter(|key| query.role.is_none_or(|role| self.role_of(*key) == Some(role)))
            .filter(|key| query.name.as_ref().is_none_or(|name| self.accessible_name(*key).is_some_and(|text| name.matches(text))))
            .filter(|key| ancestors.as_ref().is_none_or(|ancestors| self.has_ancestor_in(*key, ancestors)))
            .collect()
    }

    /// Whether a node above `key`, generic containers included, is one of `ancestors`.
    fn has_ancestor_in(&self, key: NodeKey, ancestors: &HashSet<NodeId>) -> bool {
        let mut at = NodeId(key.0);
        while let Some(parent) = self.parents.get(&at) {
            if ancestors.contains(parent) {
                return true;
            }
            at = *parent;
        }
        false
    }

    /// The nodes assistive technology reports, depth first from the root, children in their declared order: the order
    /// a screen reader reads them in.
    pub fn in_order(&self) -> Vec<NodeKey> {
        let reported: HashSet<NodeKey> = self.reported().collect();
        let mut ordered = Vec::with_capacity(reported.len());
        let mut pending: Vec<NodeKey> = self.root_key().into_iter().collect();
        while let Some(key) = pending.pop() {
            if reported.contains(&key) {
                ordered.push(key);
            }
            pending.extend(self.children_of(key).into_iter().rev());
        }
        ordered
    }

    /// The name a screen reader reads for the node, when it is not blank: its own, else the first name of the nodes
    /// that label it (`labelled_by`).
    pub fn accessible_name(&self, key: NodeKey) -> Option<&str> {
        let named = |key: NodeKey| self.name_of(key).filter(|name| !name.trim().is_empty());
        named(key).or_else(|| self.labelled_by_key(key).into_iter().find_map(named))
    }

    /// The ancestors of `key` that screen readers see, nearest first: AccessKit's consumer drops generic containers.
    pub fn ancestors(&self, key: NodeKey) -> impl Iterator<Item = NodeKey> + '_ {
        std::iter::successors(self.parent_of_key(key), |node| self.parent_of_key(*node))
    }

    /// `key` and every node below it.
    pub fn subtree(&self, key: NodeKey) -> Vec<NodeKey> {
        let mut found = vec![key];
        let mut seen: HashSet<NodeKey> = HashSet::from([key]);
        let mut at = 0;
        while at < found.len() {
            let new: Vec<NodeKey> = self.children_of(found[at]).into_iter().filter(|child| seen.insert(*child)).collect();
            found.extend(new);
            at += 1;
        }
        found
    }
}
