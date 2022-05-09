use std::{sync::Arc, vec::IntoIter};

#[derive(Debug, Default)]
pub struct GmtState {
    m1_rbm: Option<IntoIter<Vec<f64>>>,
    m2_rbm: Option<IntoIter<Vec<f64>>>,
    m1_mode: Option<IntoIter<Vec<f64>>>,
}
#[cfg(feature = "apache-arrow")]
impl From<crate::clients::arrow_client::Arrow> for GmtState {
    fn from(mut logs: crate::clients::arrow_client::Arrow) -> Self {
        Self {
            m1_rbm: logs.get("OSSM1Lc").map(|x| x.into_iter()).ok(),
            m2_rbm: logs.get("MCM2Lcl6D").map(|x| x.into_iter()).ok(),
            m1_mode: logs.get("M1modes").map(|x| x.into_iter()).ok(),
        }
    }
}
#[cfg(feature = "apache-arrow")]
impl From<(crate::clients::arrow_client::Arrow, usize, usize)> for GmtState {
    fn from((mut logs, skip, take): (crate::clients::arrow_client::Arrow, usize, usize)) -> Self {
        Self {
            m1_rbm: logs
                .get_skip_take("OSSM1Lcl", skip, take)
                .map(|x| x.into_iter())
                .ok(),
            m2_rbm: logs
                .get_skip_take("MCM2Lcl6D", skip, take)
                .map(|x| x.into_iter())
                .ok(),
            m1_mode: logs
                .get_skip_take("M1modes", skip, take)
                .map(|x| x.into_iter())
                .ok(),
        }
    }
}

impl crate::Update for GmtState {}
#[cfg(feature = "fem")]
impl crate::io::Write<Vec<f64>, fem::fem_io::OSSM1Lcl> for GmtState {
    fn write(&mut self) -> Option<std::sync::Arc<crate::io::Data<fem::fem_io::OSSM1Lcl>>> {
        self.m1_rbm
            .as_mut()
            .and_then(|x| x.next())
            .map(|x| Arc::new(crate::io::Data::new(x)))
    }
}
#[cfg(feature = "fem")]
impl crate::io::Write<Vec<f64>, fem::fem_io::MCM2Lcl6D> for GmtState {
    fn write(&mut self) -> Option<std::sync::Arc<crate::io::Data<fem::fem_io::MCM2Lcl6D>>> {
        self.m2_rbm
            .as_mut()
            .and_then(|x| x.next())
            .map(|x| Arc::new(crate::io::Data::new(x)))
    }
}
#[cfg(feature = "ceo")]
impl crate::io::Write<Vec<f64>, crate::clients::ceo::M1rbm> for GmtState {
    fn write(&mut self) -> Option<std::sync::Arc<crate::io::Data<crate::clients::ceo::M1rbm>>> {
        self.m1_rbm
            .as_mut()
            .and_then(|x| x.next())
            .map(|x| Arc::new(crate::io::Data::new(x)))
    }
}
#[cfg(feature = "ceo")]
impl crate::io::Write<Vec<f64>, crate::clients::ceo::M2rbm> for GmtState {
    fn write(&mut self) -> Option<std::sync::Arc<crate::io::Data<crate::clients::ceo::M2rbm>>> {
        self.m2_rbm
            .as_mut()
            .and_then(|x| x.next())
            .map(|x| Arc::new(crate::io::Data::new(x)))
    }
}
#[cfg(feature = "ceo")]
impl crate::io::Write<Vec<f64>, crate::clients::ceo::M1modes> for GmtState {
    fn write(&mut self) -> Option<std::sync::Arc<crate::io::Data<crate::clients::ceo::M1modes>>> {
        self.m1_mode
            .as_mut()
            .and_then(|x| x.next())
            .map(|x| Arc::new(crate::io::Data::new(x)))
    }
}
