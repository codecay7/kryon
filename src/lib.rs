/*
    FILE: lib.rs

    PURPOSE:
    Defines Kryon's reusable library modules.

    WORKFLOW:

    Kryon binary / tests
        ↓
    Library modules
        ↓
    Storage + Commands + Persistence
*/

pub mod command;
pub mod error;
pub mod persistence;
pub mod storage;
