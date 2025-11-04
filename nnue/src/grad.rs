use std::{
    mem::MaybeUninit,
    ops::{Add, Mul},
    ptr::addr_of_mut,
};

use nalgebra::{ComplexField, DMatrix, DVector, SMatrix, SVector};
use serde::{Deserialize, Serialize};

use crate::{ACC, HID, INP, OUT};

#[derive(Clone, PartialEq)]
pub struct Gradient {
    //input: Layer<INP, ACC>,
    //hidden: Layer<{ 2 * ACC }, HID>,
    //output: Layer<HID, OUT>,
    pub(crate) input_dw: DMatrix<f32>,
    pub(crate) input_db: DVector<f32>,
    pub(crate) hidden_dw: DMatrix<f32>,
    pub(crate) hidden_db: DVector<f32>,
    pub(crate) output_dw: DMatrix<f32>,
    pub(crate) output_db: DVector<f32>,
}

impl Gradient {
    pub fn zeros() -> Gradient {
        let input_dw: DMatrix<f32> = DMatrix::zeros(ACC, INP);
        let input_db: DVector<f32> = DVector::zeros(ACC);
        let hidden_dw: DMatrix<f32> = DMatrix::zeros(HID, 2 * ACC);
        let hidden_db: DVector<f32> = DVector::zeros(HID);
        let output_dw: DMatrix<f32> = DMatrix::zeros(OUT, HID);
        let output_db: DVector<f32> = DVector::zeros(OUT);
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

    pub fn adam(beta1: f32, beta2: f32, i: usize, m: &Gradient, v: &Gradient) -> Gradient {
        let mut mhat: Gradient = (1.0 / (1.0 - beta1.powi((i as i32) + 1))) * m.clone();
        let bhat: Gradient = (1.0 / (1.0 - beta2.powi((i as i32) + 1))) * v.clone();
        mhat.input_dw = mhat.input_dw.component_div(&bhat.input_dw.map(|x: f32| x.sqrt() + f32::EPSILON));
        mhat.hidden_dw = mhat.hidden_dw.component_div(&bhat.hidden_dw.map(|x: f32| x.sqrt() + f32::EPSILON));
        mhat.output_dw = mhat.output_dw.component_div(&bhat.output_dw.map(|x: f32| x.sqrt() + f32::EPSILON));

        mhat
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
        grad.input_db += &rhs.input_db;
        grad.input_dw += &rhs.input_dw;
        grad.hidden_db += &rhs.hidden_db;
        grad.hidden_dw += &rhs.hidden_dw;
        grad.output_db += &rhs.output_db;
        grad.output_dw += &rhs.output_dw;

        grad
    }
}

impl Add<&mut Gradient> for Gradient {
    type Output = Gradient;

    fn add(self, rhs: &mut Gradient) -> Self::Output {
        let mut grad = self;
        grad.input_db += &rhs.input_db;
        grad.input_dw += &rhs.input_dw;
        grad.hidden_db += &rhs.hidden_db;
        grad.hidden_dw += &rhs.hidden_dw;
        grad.output_db += &rhs.output_db;
        grad.output_dw += &rhs.output_dw;

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

impl Mul<&mut Gradient> for f32 {
    type Output = Gradient;

    fn mul(self, rhs: &mut Gradient) -> Self::Output {
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
