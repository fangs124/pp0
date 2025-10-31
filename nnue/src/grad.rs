use std::ops::{Add, Mul};

use nalgebra::{SMatrix, SVector};
use serde::{Deserialize, Serialize};

use crate::{ACC, HID, INP, OUT};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Gradient {
    input_dw: SMatrix<f32, INP, ACC>,
    input_db: SVector<f32, INP>,
    hidden_dw: SMatrix<f32, { 2 * ACC }, HID>,
    hidden_db: SVector<f32, HID>,
    output_dw: SMatrix<f32, HID, OUT>,
    output_db: SVector<f32, OUT>,
}

impl Gradient {
    pub fn zeros() -> Gradient {
        let input_dw: SMatrix<f32, INP, ACC> = SMatrix::zeros();
        let input_db: SVector<f32, INP> = SMatrix::zeros();
        let hidden_dw: SMatrix<f32, { 2 * ACC }, HID> = SMatrix::zeros();
        let hidden_db: SVector<f32, HID> = SMatrix::zeros();
        let output_dw: SMatrix<f32, HID, OUT> = SMatrix::zeros();
        let output_db: SVector<f32, OUT> = SMatrix::zeros();
        Gradient { input_dw, input_db, hidden_dw, hidden_db, output_dw, output_db }
    }

    pub fn component_square(&self) -> Gradient {
        let mut grad: Gradient = self.clone();
        grad.input_db.component_mul_assign(&self.input_db);
        grad.input_dw.component_mul_assign(&self.input_dw);
        grad.hidden_db.component_mul_assign(&self.hidden_db);
        grad.hidden_dw.component_mul_assign(&self.hidden_dw);
        grad.output_db.component_mul_assign(&self.output_db);
        grad.output_dw.component_mul_assign(&self.output_dw);

        grad
    }

    pub fn linear_sum(pairs: &mut Vec<(Gradient, f32)>) -> Gradient {
        let (mut grad, r) = pairs.pop().unwrap();
        grad.scalar_mul(r);
        let mut sum: Gradient = grad;
        for (grad, r) in pairs.iter_mut() {
            grad.scalar_mul(*r);
            sum = sum + grad;
        }

        sum
    }

    pub fn sum(grads: &mut Vec<Gradient>) -> Gradient {
        let mut sum: Gradient = grads.pop().unwrap();
        for grad in grads.iter() {
            sum = sum + grad;
        }

        sum
    }

    pub fn scalar_mul(&mut self, r: f32) {
        self.input_db *= r;
        self.input_dw *= r;
        self.hidden_db *= r;
        self.hidden_dw *= r;
        self.output_db *= r;
        self.output_dw *= r;
    }
}

impl Add<Gradient> for Gradient {
    type Output = Self;

    fn add(self, rhs: Gradient) -> Self::Output {
        let mut grad = self;
        grad.input_db += rhs.input_db;
        grad.input_dw += rhs.input_dw;
        grad.hidden_db += rhs.hidden_db;
        grad.hidden_dw += rhs.hidden_dw;
        grad.output_db += rhs.output_db;
        grad.output_dw += rhs.output_dw;

        grad
    }
}

impl Add<&Gradient> for Gradient {
    type Output = Gradient;

    fn add(self, rhs: &Gradient) -> Self::Output {
        let mut grad = self;
        grad.input_db += rhs.input_db;
        grad.input_dw += rhs.input_dw;
        grad.hidden_db += rhs.hidden_db;
        grad.hidden_dw += rhs.hidden_dw;
        grad.output_db += rhs.output_db;
        grad.output_dw += rhs.output_dw;

        grad
    }
}

impl Add<&mut Gradient> for Gradient {
    type Output = Gradient;

    fn add(self, rhs: &mut Gradient) -> Self::Output {
        let mut grad = self;
        grad.input_db += rhs.input_db;
        grad.input_dw += rhs.input_dw;
        grad.hidden_db += rhs.hidden_db;
        grad.hidden_dw += rhs.hidden_dw;
        grad.output_db += rhs.output_db;
        grad.output_dw += rhs.output_dw;

        grad
    }
}

impl Mul<Gradient> for f32 {
    type Output = Gradient;

    fn mul(self, mut rhs: Gradient) -> Self::Output {
        rhs.input_db *= self;
        rhs.input_dw *= self;
        rhs.hidden_db *= self;
        rhs.hidden_dw *= self;
        rhs.output_db *= self;
        rhs.output_dw *= self;

        rhs
    }
}

//FIXME
impl Mul<&Gradient> for f32 {
    type Output = Gradient;

    fn mul(self, rhs: &Gradient) -> Self::Output {
        let mut rhs = rhs.clone();
        rhs.input_db *= self;
        rhs.input_dw *= self;
        rhs.hidden_db *= self;
        rhs.hidden_dw *= self;
        rhs.output_db *= self;
        rhs.output_dw *= self;

        rhs
    }
}
