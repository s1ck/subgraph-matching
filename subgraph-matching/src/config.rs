use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum Filter {
    Ldf, // label-degree-filter
    Gql, // graphql-filter
    Nlf, // neighbor-label-frequency-filter
}

#[derive(Debug, Clone, Copy)]
pub enum Order {
    Gql,
}

#[derive(Debug, Clone, Copy)]
pub enum Enumeration {
    Gql,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub filter: Filter,
    pub order: Order,
    pub enumeration: Enumeration,
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Display for Enumeration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.filter, self.order, self.enumeration)
    }
}

impl Config {
    pub fn new(filter: Filter, order: Order, enumeration: Enumeration) -> Self {
        Config {
            filter,
            order,
            enumeration,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            filter: Filter::Ldf,
            order: Order::Gql,
            enumeration: Enumeration::Gql,
        }
    }
}

impl From<Filter> for Config {
    fn from(filter: Filter) -> Self {
        Config {
            filter,
            ..Config::default()
        }
    }
}

impl From<Order> for Config {
    fn from(order: Order) -> Self {
        Config {
            order,
            ..Config::default()
        }
    }
}

impl From<Enumeration> for Config {
    fn from(enumeration: Enumeration) -> Self {
        Config {
            enumeration,
            ..Config::default()
        }
    }
}
