use super::{DiscreteStateSpace, Exponential, ExponentialMatrix, Solver};
use gmt_fem::{fem_io::GetIn, fem_io::GetOut, Result, FEM};
use nalgebra as na;
use rayon::prelude::*;
use std::fmt;

/// This structure represents the actual state space model of the telescope
///
/// The state space discrete model is made of several discrete 2nd order different equation solvers, all independent and solved concurrently
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default)]
pub struct DiscreteModalSolver<T: Solver + Default> {
    /// Model input vector
    pub u: Vec<f64>,
    /// Model output vector
    pub y: Vec<f64>,
    pub y_sizes: Vec<usize>,
    /// vector of state models
    pub state_space: Vec<T>,
    /// Static gain correction matrix
    pub psi_dcg: Option<na::DMatrix<f64>>,
    /// Static gain correction vector
    pub psi_times_u: Vec<f64>,
    pub ins: Vec<Box<dyn GetIn>>,
    pub outs: Vec<Box<dyn GetOut>>,
}
impl<T: Solver + Default> DiscreteModalSolver<T> {
    /*
      /// Serializes the model using [bincode](https://docs.rs/bincode/1.3.3/bincode/)
      fn dump(&self, filename: &str) -> REs {
      let file = File::create(filename)
      }
    */
    /// Returns the FEM state space builer
    pub fn from_fem(fem: FEM) -> DiscreteStateSpace<'static, T> {
        fem.into()
    }
    /// Loads a FEM model, saved in a second order form, from a zip archive file located in a directory given by the `FEM_REPO` environment variable
    pub fn from_env() -> Result<DiscreteStateSpace<'static, T>> {
        let fem = FEM::from_env()?;
        Ok(DiscreteModalSolver::from_fem(fem))
    }
}

impl Iterator for DiscreteModalSolver<Exponential> {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.y.len();
        //        match &self.u {
        let _u_ = &self.u;
        self.y = self
            .state_space
            .par_iter_mut()
            .fold(
                || vec![0f64; n],
                |mut a: Vec<f64>, m| {
                    a.iter_mut().zip(m.solve(_u_)).for_each(|(yc, y)| {
                        *yc += y;
                    });
                    a
                },
            )
            .reduce(
                || vec![0f64; n],
                |mut a: Vec<f64>, b: Vec<f64>| {
                    a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                        *a += *b;
                    });
                    a
                },
            );
        Some(())
    }
}

impl Iterator for DiscreteModalSolver<ExponentialMatrix> {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.y.len();
        //        match &self.u {
        let _u_ = &self.u;
        self.y = self
            .state_space
            .par_iter_mut()
            .fold(
                || vec![0f64; n],
                |mut a: Vec<f64>, m| {
                    a.iter_mut().zip(m.solve(_u_)).for_each(|(yc, y)| {
                        *yc += y;
                    });
                    a
                },
            )
            .reduce(
                || vec![0f64; n],
                |mut a: Vec<f64>, b: Vec<f64>| {
                    a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                        *a += *b;
                    });
                    a
                },
            );

        if let Some(psi_dcg) = &self.psi_dcg {
            self.y
                .iter_mut()
                .zip(&self.psi_times_u)
                .for_each(|(v1, v2)| *v1 += *v2);
            let u_nalgebra = na::DVector::from_column_slice(&self.u);
            self.psi_times_u = (psi_dcg * u_nalgebra).as_slice().to_vec();
        }

        Some(())
    }
}
impl<T: Solver + Default> fmt::Display for DiscreteModalSolver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r##"
DiscreteModalSolver:
 - inputs ({}):
{:}
 - outputs ({}):
{:}
 - {:} 2x2 state space models
"##,
            self.u.len(),
            self.ins
                .iter()
                .map(|x| x.fem_type())
                .collect::<Vec<String>>()
                .join("\n"),
            self.y.len(),
            self.outs
                .iter()
                .map(|x| x.fem_type())
                .collect::<Vec<String>>()
                .join("\n"),
            self.state_space.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fem_io::actors_inputs::OSSElDriveTorque;
    use crate::fem_io::actors_outputs::OSSElEncoderAngle;
    use gmt_fem::FEM;

    #[test]
    fn serde() {
        let state_space = {
            let fem = FEM::from_env().unwrap();
            DiscreteModalSolver::<ExponentialMatrix>::from_fem(fem)
                .sampling(1e3)
                .max_eigen_frequency(0.1)
                .ins::<OSSElDriveTorque>()
                .outs::<OSSElEncoderAngle>()
                .build()
                .unwrap()
        };
        dbg!(&state_space);
        
        let json = serde_json::to_string(&state_space).unwrap();
        println!("{:#}", &json);
        let q: DiscreteModalSolver<ExponentialMatrix> = serde_json::from_str(&json).unwrap();
        dbg!(&q);
    }
}
