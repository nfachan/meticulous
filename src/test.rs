#![cfg(test)]

macro_rules! ceid {
    [$n:expr] => {
        crate::ClientExecutionId($n)
    };
}
pub(crate) use ceid;

macro_rules! cid {
    [$n:expr] => { crate::ClientId($n) };
}
pub(crate) use cid;

macro_rules! wid {
    [$n:expr] => { crate::WorkerId($n) };
}
pub(crate) use wid;

macro_rules! eid {
    [$n:expr] => {
        eid!($n, $n)
    };
    [$cid:expr, $ceid:expr] => {
        $crate::ExecutionId(cid![$cid], ceid![$ceid])
    };
}
pub(crate) use eid;

macro_rules! details {
    [1] => {
        $crate::ExecutionDetails {
            program: "test_1".to_string(),
            arguments: vec![],
        }
    };
    [2] => {
        $crate::ExecutionDetails {
            program: "test_2".to_string(),
            arguments: vec!["arg_1".to_string()],
        }
    };
    [3] => {
        $crate::ExecutionDetails {
            program: "test_3".to_string(),
            arguments: vec!["arg_1".to_string(), "arg_2".to_string()],
        }
    };
    [4] => {
        $crate::ExecutionDetails {
            program: "test_4".to_string(),
            arguments: vec!["arg_1".to_string(), "arg_2".to_string(), "arg_3".to_string()],
        }
    };
    [$n:literal] => {
        $crate::ExecutionDetails {
            program: concat!("test_", stringify!($n)).to_string(),
            arguments: vec!["arg_1".to_string()],
        }
    };
}
pub(crate) use details;

macro_rules! result {
    [1] => {
        $crate::ExecutionResult::Exited(0)
    };
    [2] => {
        $crate::ExecutionResult::Exited(1)
    };
    [3] => {
        $crate::ExecutionResult::Signalled(15)
    };
    [$n:expr] => {
        $crate::ExecutionResult::Exited($n)
    };
}
pub(crate) use result;
