//! This module is used to build the state space model of the telescope structure
//!
//! A state space model is represented by the structure [DiscreteModalSolver] that is created using the builder [`DiscreteStateSpace`].
//! The transformation of the FEM continuous 2nd order differential equation
//! into a discrete state space model is performed by the [Exponential] structure
//! (for the details of the transformation see the `exponential` module ).
//!
//! # Example
//! The following example loads a FEM model and converts it into a state space model
//! setting the sampling rate and the damping coefficients and truncating the eigen frequencies.
//! A single input and a single output are selected.
//! ```no_run
//! use gmt_fem::{FEM,
//!               dos::{DiscreteStateSpace, DiscreteModalSolver, Exponential},
//!               fem_io::{OSSM1Lcl6F, OSSM1Lcl}};
//!
//! # fn main() -> anyhow::Result<()> {
//!     let sampling_rate = 1e3; // Hz
//!     let fem = FEM::from_env()?;
//!     let mut fem_ss: DiscreteModalSolver<Exponential> = DiscreteStateSpace::from(fem)
//!         .sampling(sampling_rate)
//!         .proportional_damping(2. / 100.)
//!         .max_eigen_frequency(75.0) // Hz
//!         .ins::<OSSM1Lcl6F>()
//!         .outs::<OSSM1Lcl>()
//!         .build()?;
//! # Ok::<(), anyhow::Error>(())
//! # }
//! ```

use gmt_dos_clients::interface::UniqueIdentifier;
use gmt_fem::fem_io;
use std::{fmt::Debug, ops::Range};

mod bilinear;
pub use bilinear::Bilinear;
mod exponential;
pub use exponential::Exponential;
mod exponential_matrix;
pub use exponential_matrix::ExponentialMatrix;
mod discrete_state_space;
pub use discrete_state_space::DiscreteStateSpace;
mod discrete_modal_solver;
pub use discrete_modal_solver::DiscreteModalSolver;

pub mod actors_interface;

pub trait Solver {
    fn from_second_order(
        tau: f64,
        omega: f64,
        zeta: f64,
        continuous_bb: Vec<f64>,
        continuous_cc: Vec<f64>,
    ) -> Self;
    fn solve(&mut self, u: &[f64]) -> &[f64];
}

#[derive(Debug, thiserror::Error)]
pub enum StateSpaceError {
    #[error("argument {0} is missing")]
    MissingArguments(String),
    #[error("sampling frequency not set")]
    SamplingFrequency,
    #[error("{0}")]
    Matrix(String),
    #[error("FEM IO error")]
    FemIO(#[from] gmt_fem::FemError),
}

type Result<T> = std::result::Result<T, StateSpaceError>;

pub trait Get<U: UniqueIdentifier> {
    fn get(&self) -> Option<Vec<f64>>;
}
impl<T, U> Get<U> for DiscreteModalSolver<T>
where
    Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
    T: Solver + Default,
    U: 'static + UniqueIdentifier,
{
    fn get(&self) -> Option<Vec<f64>> {
        self.outs
            .iter()
            .find(|&x| x.as_any().is::<fem_io::SplitFem<U>>())
            .map(|io| self.y[io.range()].to_vec())
    }
}
pub trait Set<U: UniqueIdentifier> {
    fn set(&mut self, u: &[f64]);
    fn set_slice(&mut self, _u: &[f64], _range: Range<usize>) {
        unimplemented!()
    }
}
impl<T, U> Set<U> for DiscreteModalSolver<T>
where
    Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
    T: Solver + Default,
    U: 'static + UniqueIdentifier,
{
    fn set(&mut self, u: &[f64]) {
        if let Some(io) = self
            .ins
            .iter()
            .find(|&x| x.as_any().is::<fem_io::SplitFem<U>>())
        {
            self.u[io.range()].copy_from_slice(u);
        }
    }
    fn set_slice(&mut self, u: &[f64], range: Range<usize>) {
        if let Some(io) = self
            .ins
            .iter()
            .find(|&x| x.as_any().is::<fem_io::SplitFem<U>>())
        {
            self.u[io.range()][range].copy_from_slice(u);
        }
    }
}
