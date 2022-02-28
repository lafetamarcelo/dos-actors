use dos_actors::{
    io::{Data, Write},
    Update,
};
use std::sync::Arc;

pub struct Signal {
    pub sampling_frequency: f64,
    pub period: f64,
    pub n_step: usize,
    pub step: usize,
    pub value: Option<f64>,
}
impl Update for Signal {
    fn update(&mut self) {
        self.value = {
            if self.step < self.n_step {
                let value = (2.
                    * std::f64::consts::PI
                    * self.step as f64
                    * (self.sampling_frequency * self.period).recip())
                .sin()
                    - 0.25
                        * (2.
                            * std::f64::consts::PI
                            * ((self.step as f64
                                * (self.sampling_frequency * self.period * 0.25).recip())
                                + 0.1))
                            .sin();
                self.step += 1;
                Some(value)
            } else {
                None
            }
        };
    }
}

pub enum SignalToFilter {}
impl Write<f64, SignalToFilter> for Signal {
    fn write(&self) -> Option<Arc<Data<f64, SignalToFilter>>> {
        self.value.map(|x| Arc::new(Data::new(x)))
    }
}
