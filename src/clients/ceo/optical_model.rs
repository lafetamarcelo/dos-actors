use crate::{
    io::{Data, Read, Write},
    Update,
};
use crseo::{
    pssn::{AtmosphereTelescopeError, TelescopeError},
    Atmosphere, Builder, Diffractive, Geometric, Gmt, PSSnEstimates, ShackHartmannBuilder, Source,
    WavefrontSensor, WavefrontSensorBuilder, ATMOSPHERE, GMT, PSSN, SOURCE,
};
use nalgebra as na;
use std::{ops::DerefMut, sync::Arc};

#[derive(thiserror::Error, Debug)]
pub enum CeoError {
    #[error("CEO building failed")]
    CEO(#[from] crseo::CrseoError),
}
pub type Result<T> = std::result::Result<T, CeoError>;

/// Shack-Hartmann wavefront sensor type: [Diffractive] or [Geometric]
#[derive(PartialEq)]
pub enum ShackHartmannOptions {
    Diffractive(ShackHartmannBuilder<Diffractive>),
    Geometric(ShackHartmannBuilder<Geometric>),
}
/// PSSn model
#[derive(PartialEq)]
pub enum PSSnOptions {
    Telescope(PSSN<TelescopeError>),
    AtmosphereTelescope(PSSN<AtmosphereTelescopeError>),
}
/// Options for [OpticalModelBuilder]
#[derive(PartialEq)]
pub enum OpticalModelOptions {
    Atmosphere {
        builder: ATMOSPHERE,
        time_step: f64,
    },
    ShackHartmann {
        options: ShackHartmannOptions,
        flux_threshold: f64,
    },
    PSSn(PSSnOptions),
}

/// GMT optical model builder
pub struct OpticalModelBuilder {
    gmt: GMT,
    src: SOURCE,
    options: Option<Vec<OpticalModelOptions>>,
}
impl Default for OpticalModelBuilder {
    fn default() -> Self {
        Self {
            gmt: GMT::default(),
            src: SOURCE::default(),
            options: None,
        }
    }
}

pub trait SensorBuilder: WavefrontSensorBuilder + Builder + Clone {
    fn build(
        self,
        gmt_builder: GMT,
        src_builder: SOURCE,
        threshold: f64,
    ) -> Result<Box<dyn WavefrontSensor>>;
}

impl OpticalModelBuilder {
    /// Creates a new GMT optical model
    ///
    /// Creates a default builder based on the default parameters for [GMT] and [SOURCE]
    pub fn new() -> Self {
        Default::default()
    }
    /// Sets the GMT builder
    pub fn gmt(self, gmt: GMT) -> Self {
        Self { gmt, ..self }
    }
    /// Sets the `Source` builder
    pub fn source(self, src: SOURCE) -> Self {
        Self { src, ..self }
    }
    /// Sets [OpticalModel] [options](OpticalModelOptions)
    pub fn options(self, options: Vec<OpticalModelOptions>) -> Self {
        Self {
            options: Some(options),
            ..self
        }
    }
    /// Builds a new GMT optical model
    ///
    /// If there is `Some` sensor, it is initialized.

    pub fn build(self) -> Result<OpticalModel> {
        let src = self.src.clone().build()?;
        let gmt = self.gmt.clone().build()?;
        let mut optical_model = OpticalModel {
            gmt,
            src,
            sensor: None,
            atm: None,
            pssn: None,
            sensor_fn: SensorFn::None,
            frame: None,
            tau: 0f64,
        };
        if let Some(options) = self.options {
            options.into_iter().for_each(|option| match option {
                OpticalModelOptions::PSSn(PSSnOptions::Telescope(pssn_builder)) => {
                    optical_model.pssn = pssn_builder
                        .source(&(self.src.clone().build().unwrap()))
                        .build()
                        .ok()
                        .map(|x| Box::new(x) as Box<dyn PSSnEstimates>);
                }
                OpticalModelOptions::PSSn(PSSnOptions::AtmosphereTelescope(pssn_builder)) => {
                    optical_model.pssn = pssn_builder
                        .source(&(self.src.clone().build().unwrap()))
                        .build()
                        .ok()
                        .map(|x| Box::new(x) as Box<dyn PSSnEstimates>);
                }
                OpticalModelOptions::Atmosphere { builder, time_step } => {
                    optical_model.atm = builder.build().ok();
                    optical_model.tau = time_step;
                }
                OpticalModelOptions::ShackHartmann {
                    options,
                    flux_threshold,
                } => match options {
                    ShackHartmannOptions::Diffractive(sensor_builder) => {
                        optical_model.src = sensor_builder
                            .guide_stars(Some(self.src.clone()))
                            .build()
                            .unwrap();
                        optical_model.sensor = SensorBuilder::build(
                            sensor_builder,
                            self.gmt.clone(),
                            self.src.clone(),
                            flux_threshold,
                        )
                        .ok();
                    }
                    ShackHartmannOptions::Geometric(sensor_builder) => {
                        optical_model.src = sensor_builder
                            .guide_stars(Some(self.src.clone()))
                            .build()
                            .unwrap();
                        optical_model.sensor = SensorBuilder::build(
                            sensor_builder,
                            self.gmt.clone(),
                            self.src.clone(),
                            flux_threshold,
                        )
                        .ok();
                    }
                },
            });
        }
        Ok(optical_model)
    }
}
pub enum SensorFn {
    None,
    Fn(Box<dyn Fn(Vec<f64>) -> Vec<f64> + Send>),
    Matrix(na::DMatrix<f64>),
}
/// GMT Optical Model
pub struct OpticalModel {
    pub gmt: Gmt,
    pub src: Source,
    pub sensor: Option<Box<dyn WavefrontSensor>>,
    pub atm: Option<Atmosphere>,
    pub pssn: Option<Box<dyn PSSnEstimates>>,
    pub sensor_fn: SensorFn,
    pub(crate) frame: Option<Vec<f32>>,
    tau: f64,
}
impl OpticalModel {
    pub fn builder() -> OpticalModelBuilder {
        OpticalModelBuilder::new()
    }
    pub fn sensor_matrix_transform(&mut self, mat: na::DMatrix<f64>) -> &mut Self {
        self.sensor_fn = SensorFn::Matrix(mat);
        self
    }
}

impl Update for OpticalModel {
    fn update(&mut self) {
        self.src.through(&mut self.gmt).xpupil();
        if let Some(atm) = &mut self.atm {
            atm.secs += self.tau;
            self.src.through(atm);
        }
        if let Some(sensor) = &mut self.sensor {
            //self.src.through(sensor);
            sensor.deref_mut().propagate(&mut self.src);
        }
        if let Some(pssn) = &mut self.pssn {
            self.src.through(pssn);
        }
    }
}

impl crate::clients::TimerMarker for OpticalModel {}

#[cfg(feature = "crseo")]
impl Read<crseo::gmt::SegmentsDof, super::GmtState> for OpticalModel {
    fn read(&mut self, data: Arc<Data<super::GmtState>>) {
        if let Err(e) = &data.apply_to(&mut self.gmt) {
            crate::print_error("Failed applying GMT state", e);
        }
    }
}
impl Read<Vec<f64>, super::M1rbm> for OpticalModel {
    fn read(&mut self, data: Arc<Data<super::M1rbm>>) {
        data.chunks(6).enumerate().for_each(|(sid0, v)| {
            self.gmt
                .m1_segment_state((sid0 + 1) as i32, &v[..3], &v[3..]);
        });
    }
}
impl Read<Vec<f64>, super::M1modes> for OpticalModel {
    fn read(&mut self, data: Arc<Data<super::M1modes>>) {
        self.gmt.m1_modes(&data);
    }
}
impl Read<Vec<f64>, super::M2rbm> for OpticalModel {
    fn read(&mut self, data: Arc<Data<super::M2rbm>>) {
        data.chunks(6).enumerate().for_each(|(sid0, v)| {
            self.gmt
                .m2_segment_state((sid0 + 1) as i32, &v[..3], &v[3..]);
        });
    }
}
#[cfg(feature = "fem")]
impl Read<Vec<f64>, fem::fem_io::OSSM1Lcl> for OpticalModel {
    fn read(&mut self, data: Arc<Data<fem::fem_io::OSSM1Lcl>>) {
        data.chunks(6).enumerate().for_each(|(sid0, v)| {
            self.gmt
                .m1_segment_state((sid0 + 1) as i32, &v[..3], &v[3..]);
        });
    }
}
#[cfg(feature = "fem")]
impl Read<Vec<f64>, fem::fem_io::MCM2Lcl6D> for OpticalModel {
    fn read(&mut self, data: Arc<Data<fem::fem_io::MCM2Lcl6D>>) {
        data.chunks(6).enumerate().for_each(|(sid0, v)| {
            self.gmt
                .m2_segment_state((sid0 + 1) as i32, &v[..3], &v[3..]);
        });
    }
}
impl Write<Vec<f64>, super::WfeRms> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::WfeRms>>> {
        Some(Arc::new(Data::new(self.src.wfe_rms())))
    }
}
impl Write<Vec<f64>, super::TipTilt> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::TipTilt>>> {
        Some(Arc::new(Data::new(self.src.gradients())))
    }
}
impl Write<Vec<f64>, super::SegmentWfeRms> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::SegmentWfeRms>>> {
        Some(Arc::new(Data::new(self.src.segment_wfe_rms())))
    }
}
impl Write<Vec<f64>, super::SegmentPiston> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::SegmentPiston>>> {
        Some(Arc::new(Data::new(self.src.segment_piston())))
    }
}
impl Write<Vec<f64>, super::SegmentGradients> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::SegmentGradients>>> {
        Some(Arc::new(Data::new(self.src.segment_gradients())))
    }
}
impl Write<Vec<f64>, super::SegmentTipTilt> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::SegmentTipTilt>>> {
        Some(Arc::new(Data::new(self.src.segment_gradients())))
    }
}
impl Write<Vec<f64>, super::PSSn> for OpticalModel {
    fn write(&mut self) -> Option<Arc<Data<super::PSSn>>> {
        self.pssn
            .as_mut()
            .map(|pssn| Arc::new(Data::new(pssn.estimates())))
    }
}
