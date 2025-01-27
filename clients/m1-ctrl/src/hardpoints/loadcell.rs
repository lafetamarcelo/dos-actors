use gmt_dos_clients::interface::{Data, Read, Size, Update, Write};
use gmt_dos_clients_io::gmt_m1::segment;

type M = nalgebra::Matrix6<f64>;
type V = nalgebra::Vector6<f64>;

/// [gmt_dos_actors](https://docs.rs/gmt_dos-actors) client interface for hardpoints loadcells
#[derive(Debug, Clone)]
pub struct LoadCells {
    pub(super) hp_f_cmd: Vec<f64>,
    pub(super) hp_d_cell: Vec<f64>,
    pub(super) hp_d_face: Vec<f64>,
    hp_f_meas: Vec<f64>,
    m1_hpk: f64,
    lc_2_cg: M,
}
impl LoadCells {
    /// Creates a new loadcells client
    ///
    /// The hardpoints stiffness and the matrix transformation
    /// from local to center of gravity coordinates are provided.
    pub fn new(m1_hpk: f64, lc_2_cg: M) -> Self {
        Self {
            m1_hpk,
            hp_f_cmd: vec![0f64; 6],
            hp_d_cell: vec![0f64; 6],
            hp_d_face: vec![0f64; 6],
            hp_f_meas: vec![0f64; 6],
            lc_2_cg,
        }
    }
}

impl<const ID: u8> Size<segment::HardpointsMotion<ID>> for LoadCells {
    fn len(&self) -> usize {
        12
    }
}

impl<const ID: u8> Size<segment::BarycentricForce<ID>> for LoadCells {
    fn len(&self) -> usize {
        6
    }
}

impl Update for LoadCells {
    fn update(&mut self) {
        self.hp_d_cell
            .iter()
            .zip(self.hp_d_face.iter())
            .map(|(hp_d_cell, hp_d_face)| hp_d_face - hp_d_cell)
            .map(|hp_relative_displacements| hp_relative_displacements * self.m1_hpk)
            .zip(self.hp_f_cmd.iter())
            .map(|(hp_relative_force, hp_f_cmd)| hp_relative_force - hp_f_cmd)
            .zip(&mut self.hp_f_meas)
            .for_each(|(hp_f_diff_force, hp_f_meas)| *hp_f_meas = hp_f_diff_force);
    }
}

impl<const ID: u8> Read<segment::HardpointsForces<ID>> for LoadCells {
    fn read(&mut self, data: Data<segment::HardpointsForces<ID>>) {
        self.hp_f_cmd = (**data).to_vec();
    }
}

impl<const ID: u8> Read<segment::HardpointsMotion<ID>> for LoadCells {
    fn read(&mut self, data: Data<segment::HardpointsMotion<ID>>) {
        let (cell, face) = (&data).split_at(6);
        self.hp_d_cell.copy_from_slice(cell);
        self.hp_d_face.copy_from_slice(face);
    }
}

impl<const ID: u8> Write<segment::BarycentricForce<ID>> for LoadCells {
    fn write(&mut self) -> Option<Data<segment::BarycentricForce<ID>>> {
        let cg = self.lc_2_cg * V::from_column_slice(self.hp_f_meas.as_slice());
        Some(Data::new(cg.as_slice().to_vec()))
    }
}
