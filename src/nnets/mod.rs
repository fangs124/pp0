extern crate nalgebra as na;

mod grad;
mod phi;

use std::ops::{Add, Mul};

use na::base::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use grad::Gradient;
use phi::PhiT;

use crate::nnets::phi::safesoftmax;

#[derive(Serialize, Deserialize, Clone)]
pub struct Network {
    pub input_dim: usize,
    pub node_counts: Vec<usize>,
    layers: Vec<Layer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Layer {
    w: DMatrix<f32>,
    b: DVector<f32>,
    z: DVector<f32>, //z = w*phi(z') + b
    index: usize,
    ty: PhiT,
}

pub trait InputType {
    fn to_vector(&self) -> DVector<f32>;
}

impl Network {
    const TRESHOLD: f32 = 0.0005;
    //const DEFAULT_ALPHA: f32 = 0.5;
    //const DEFAULT_GAMMA: f32 = 0.90;
    const DEFAULT_IN_PHI: PhiT = PhiT::LReLU6;
    const DEFAULT_OUT_PHI: PhiT = PhiT::Tanh;

    pub fn z(&self) -> Vec<f32> {
        let i = self.layers.len() - 1;
        self.layers[i].z.data.as_vec().to_vec()
    }

    pub fn phi_z(&self) -> Vec<f32> {
        let i = self.layers.len() - 1;
        self.layers[i].z.map(&self.layers[i].ty.phi()).data.as_vec().to_vec()
    }

    // last layer uses pi(phi(z))
    // so dpi/dz = dpi/dphi * dphi/dz
    pub fn pi(&self) -> Vec<f32> {
        let i: usize = self.layers.len() - 1;
        safesoftmax(self.layers[i].z.map(&PhiT::phi(&self.layers[i].ty)).data.as_vec())
    }

    pub fn new(input_dim: usize, node_counts: Vec<usize>) -> Self {
        let mut layers: Vec<Layer> = Vec::with_capacity(node_counts.len());
        let mut j = input_dim;
        for (nth, &i) in node_counts.iter().enumerate() {
            layers.push(Layer::new(i, j, nth, Network::DEFAULT_IN_PHI));
            j = i;
        }

        layers[node_counts.len() - 1].ty = Network::DEFAULT_OUT_PHI;
        Network { input_dim, node_counts, layers }
    }

    pub fn forward_prop<S>(&mut self, input: &impl InputType) {
        //
        let mut prev_phiz = input.to_vector();
        for layer in self.layers.iter_mut() {
            layer.compute_z(&prev_phiz);
            prev_phiz = layer.phi();
        }
    }

    pub fn backward_prop<S>(&mut self, input: &impl InputType) -> Gradient {
        let phi_vec = self.phi_z();
        let input_vector = input.to_vector();
        // dphi/da
        let mut dphida = self.layers[self.layers.len() - 1].dphi();
        let mut grad = Gradient::new();
        for layer in self.layers.iter().rev() {
            // dphi/dz = dphi/da * da/dz
            let dphidz = dphida.component_mul(&dphida);
            grad.dbs.push(dphidz);

            // dz/dw
            let dzdw = match layer.index {
                0 => input_vector.clone(),
                _ => self.layers[layer.index - 1].phi(),
            };

            grad.dws.push(&grad.dbs[grad.dbs.len() - 1] * dzdw.transpose());
            dphida = layer.w.tr_mul(&grad.dbs[grad.dbs.len() - 1]);
        }

        grad.dbs.reverse();
        grad.dws.reverse();
        for (db, dw) in grad.dbs.iter().zip(&grad.dws) {
            grad.dbs_shape.push(db.len());
            grad.dws_shape.push(dw.shape());
        }

        return grad;
    }
}

impl Layer {
    pub fn new(i: usize, j: usize, index: usize, phi: PhiT) -> Self {
        let w = DMatrix::new_random(i, j);
        let b = DVector::new_random(i);
        let z = DVector::zeros(i);
        Layer { w, b, z, index, ty: phi }
    }

    pub fn compute_z(&mut self, prev_phiz: &DVector<f32>) {
        self.z = &self.w * prev_phiz + &self.b;
    }

    pub fn phi(&self) -> DVector<f32> {
        self.z.map(self.ty.phi())
    }

    pub fn dphi(&self) -> DVector<f32> {
        self.z.map(self.ty.dphi())
    }
}
