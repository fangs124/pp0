use std::{iter::zip, ops::Add};

use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use crate::nnets::Network;

#[derive(Serialize, Deserialize, Clone)]
pub struct Gradient {
    pub dbs: Vec<DVector<f32>>,
    pub dws: Vec<DMatrix<f32>>,
    pub dbs_shape: Vec<usize>,
    pub dws_shape: Vec<(usize, usize)>,
}

impl Gradient {
    pub fn new() -> Self {
        Gradient { dbs: Vec::new(), dbs_shape: Vec::new(), dws: Vec::new(), dws_shape: Vec::new() }
    }

    pub fn zero(net: &Network) -> Self {
        let mut grad = Gradient::new();
        for layer in &net.layers {
            grad.dbs.push(DVector::zeros(layer.b.nrows()));
            grad.dbs_shape.push(layer.b.nrows());
            grad.dws.push(DMatrix::zeros(layer.w.nrows(), layer.w.ncols()));
            grad.dws_shape.push((layer.w.nrows(), layer.w.ncols()));
        }
        return grad;
    }

    pub fn sum(grads: &mut Vec<Gradient>) -> Gradient {
        let mut sum: Gradient = grads.pop().unwrap();
        for grad in grads.iter() {
            sum = sum + grad;
        }
        return sum;
    }
}

impl Add<Gradient> for Gradient {
    type Output = Self;

    fn add(self, rhs: Gradient) -> Self::Output {
        assert!((self.dbs_shape == rhs.dbs_shape) && (self.dws_shape == rhs.dws_shape));
        let mut Gradient = Gradient::new();
        Gradient.dbs_shape = self.dbs_shape;
        Gradient.dws_shape = self.dws_shape;
        for (db_l, db_r) in zip(self.dbs, rhs.dbs) {
            Gradient.dbs.push(db_l + db_r);
        }
        for (dw_l, dw_r) in zip(self.dws, rhs.dws) {
            Gradient.dws.push(dw_l + dw_r);
        }
        return Gradient;
    }
}

//FIXME
impl std::ops::Add<&Gradient> for Gradient {
    type Output = Gradient;

    fn add(self, rhs: &Gradient) -> Self::Output {
        return self + rhs.clone();
    }
}
