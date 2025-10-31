use std::{
    mem::MaybeUninit,
    ops::{Add, Mul},
    ptr::addr_of_mut,
};

use nalgebra::{ComplexField, SMatrix, SVector};
use serde::{Deserialize, Serialize};

use crate::{ACC, HID, INP, NetworkData, OUT};

#[derive(Clone, PartialEq)]
pub struct Gradient {
    pub(crate) data: Box<GradientData>,
}

#[derive(Clone, PartialEq)]
pub struct GradientData {
    //input: Layer<INP, ACC>,
    //hidden: Layer<{ 2 * ACC }, HID>,
    //output: Layer<HID, OUT>,
    pub(crate) input_dw: SMatrix<f32, ACC, INP>,
    pub(crate) input_db: SVector<f32, ACC>,
    pub(crate) hidden_dw: SMatrix<f32, HID, { 2 * ACC }>,
    pub(crate) hidden_db: SVector<f32, HID>,
    pub(crate) output_dw: SMatrix<f32, OUT, HID>,
    pub(crate) output_db: SVector<f32, OUT>,
}

impl Gradient {
    pub fn zeros() -> Gradient {
        Gradient { data: GradientData::zeros() }
    }

    pub fn component_square(&self) -> Gradient {
        let mut grad: Gradient = self.clone();
        grad.data.input_db.component_mul_assign(&self.data.input_db);
        grad.data.input_dw.component_mul_assign(&self.data.input_dw);
        grad.data.hidden_db.component_mul_assign(&self.data.hidden_db);
        grad.data.hidden_dw.component_mul_assign(&self.data.hidden_dw);
        grad.data.output_db.component_mul_assign(&self.data.output_db);
        grad.data.output_dw.component_mul_assign(&self.data.output_dw);

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
        self.data.input_db *= r;
        self.data.input_dw *= r;
        self.data.hidden_db *= r;
        self.data.hidden_dw *= r;
        self.data.output_db *= r;
        self.data.output_dw *= r;
    }

    pub fn adam(beta1: f32, beta2: f32, i: usize, m: &Gradient, v: &Gradient) -> Gradient {
        let mut mhat: Gradient = (1.0 / (1.0 - beta1.powi((i as i32) + 1))) * m.clone();
        let bhat: Gradient = (1.0 / (1.0 - beta2.powi((i as i32) + 1))) * v.clone();
        mhat.data.input_dw = mhat.data.input_dw.component_div(&bhat.data.input_dw.map(|x: f32| x.sqrt() + f32::EPSILON));
        mhat.data.hidden_dw = mhat.data.hidden_dw.component_div(&bhat.data.hidden_dw.map(|x: f32| x.sqrt() + f32::EPSILON));
        mhat.data.output_dw = mhat.data.output_dw.component_div(&bhat.data.output_dw.map(|x: f32| x.sqrt() + f32::EPSILON));

        mhat
    }
}

impl GradientData {
    fn zeros() -> Box<GradientData> {
        let input_dw: Box<SMatrix<f32, ACC, INP>> = Box::new(SMatrix::zeros());
        let input_db: Box<SVector<f32, ACC>> = Box::new(SVector::zeros());
        let hidden_dw: Box<SMatrix<f32, HID, { 2 * ACC }>> = Box::new(SMatrix::zeros());
        let hidden_db: Box<SVector<f32, HID>> = Box::new(SVector::zeros());
        let output_dw: Box<SMatrix<f32, OUT, HID>> = Box::new(SMatrix::zeros());
        let output_db: Box<SVector<f32, OUT>> = Box::new(SVector::zeros());

        #[rustfmt::skip]
        let gradient_data = {
            let mut gradient: Box<MaybeUninit<GradientData>> = Box::new_uninit();
            let ptr = gradient.as_mut_ptr();
            unsafe { addr_of_mut!((*ptr).input_dw).write(*input_dw); }
            unsafe { addr_of_mut!((*ptr).input_db).write(*input_db); }
            unsafe { addr_of_mut!((*ptr).hidden_dw).write(*hidden_dw); }
            unsafe { addr_of_mut!((*ptr).hidden_db).write(*hidden_db); }
            unsafe { addr_of_mut!((*ptr).output_dw).write(*output_dw); }
            unsafe { addr_of_mut!((*ptr).output_db).write(*output_db); }
            unsafe { gradient.assume_init() }
        };

        gradient_data
    }
}
impl Add<Gradient> for Gradient {
    type Output = Self;

    fn add(self, rhs: Gradient) -> Self::Output {
        let mut grad = self;
        grad.data.input_db += rhs.data.input_db;
        grad.data.input_dw += rhs.data.input_dw;
        grad.data.hidden_db += rhs.data.hidden_db;
        grad.data.hidden_dw += rhs.data.hidden_dw;
        grad.data.output_db += rhs.data.output_db;
        grad.data.output_dw += rhs.data.output_dw;

        grad
    }
}

impl Add<&Gradient> for Gradient {
    type Output = Gradient;

    fn add(self, rhs: &Gradient) -> Self::Output {
        let mut grad = self;
        grad.data.input_db += rhs.data.input_db;
        grad.data.input_dw += rhs.data.input_dw;
        grad.data.hidden_db += rhs.data.hidden_db;
        grad.data.hidden_dw += rhs.data.hidden_dw;
        grad.data.output_db += rhs.data.output_db;
        grad.data.output_dw += rhs.data.output_dw;

        grad
    }
}

impl Add<&mut Gradient> for Gradient {
    type Output = Gradient;

    fn add(self, rhs: &mut Gradient) -> Self::Output {
        let mut grad = self;
        grad.data.input_db += rhs.data.input_db;
        grad.data.input_dw += rhs.data.input_dw;
        grad.data.hidden_db += rhs.data.hidden_db;
        grad.data.hidden_dw += rhs.data.hidden_dw;
        grad.data.output_db += rhs.data.output_db;
        grad.data.output_dw += rhs.data.output_dw;

        grad
    }
}

impl Mul<Gradient> for f32 {
    type Output = Gradient;

    fn mul(self, mut rhs: Gradient) -> Self::Output {
        rhs.data.input_db *= self;
        rhs.data.input_dw *= self;
        rhs.data.hidden_db *= self;
        rhs.data.hidden_dw *= self;
        rhs.data.output_db *= self;
        rhs.data.output_dw *= self;

        rhs
    }
}

//FIXME
impl Mul<&Gradient> for f32 {
    type Output = Gradient;

    fn mul(self, rhs: &Gradient) -> Self::Output {
        let mut rhs = rhs.clone();
        rhs.data.input_db *= self;
        rhs.data.input_dw *= self;
        rhs.data.hidden_db *= self;
        rhs.data.hidden_dw *= self;
        rhs.data.output_db *= self;
        rhs.data.output_dw *= self;

        rhs
    }
}
