use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum PhiT {
    Sigmoid,
    ReLU,
    //ReLU6,
    LReLU,
    LReLU6,
    CReLU,
    Tanh,
    Id,
    //SoftPlus,
    //FSigmoid,
    //Linear,
    //PReLU,
    //SPReLU,
}

impl PhiT {
    pub fn phi(&self) -> fn(f32) -> f32 {
        match self {
            PhiT::Sigmoid => sigmoid,
            PhiT::ReLU => relu,
            PhiT::LReLU => lrelu,
            PhiT::LReLU6 => lrelu6,
            PhiT::CReLU => crelu,
            PhiT::Tanh => tanh,
            PhiT::Id => id,
        }
    }

    pub fn dphi(&self) -> fn(f32) -> f32 {
        match self {
            PhiT::Sigmoid => dsigmoid,
            PhiT::ReLU => drelu,
            PhiT::LReLU => dlrelu,
            PhiT::LReLU6 => dlrelu6,
            PhiT::CReLU => dcrelu,
            PhiT::Tanh => dtanh,
            PhiT::Id => did,
        }
    }
}

fn id(x: f32) -> f32 {
    x
}

fn did(_x: f32) -> f32 {
    1.0
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + f32::exp(-x))
}

fn dsigmoid(x: f32) -> f32 {
    sigmoid(x) * (1.0 - sigmoid(x))
}

fn relu(x: f32) -> f32 {
    if x >= 0.0 { x } else { 0.0 }
}

fn drelu(x: f32) -> f32 {
    if x >= 0.0 { 1.0 } else { 0.0 }
}

fn lrelu(x: f32) -> f32 {
    if x < 0.0 { 0.01 * x } else { x }
}

fn dlrelu(x: f32) -> f32 {
    if x < 0.0 { 0.01 } else { 1.0 }
}

fn crelu(x: f32) -> f32 {
    x.min(1.0).max(0.0)
}
fn dcrelu(x: f32) -> f32 {
    if x < 0.0 {
        0.00001
    } else if x <= 1.0 {
        1.0
    } else {
        0.00001
    }
}

//fn screlu(x: f32) -> f32 {
//    todo!();
//}
//fn dscrelu(x: f32) -> f32 {
//    todo!();
//}

fn lrelu6(x: f32) -> f32 {
    if x < 0.0 {
        0.01 * x
    } else if x <= 6.0 {
        x
    } else {
        6.0 + 0.01 * x
    }
}

fn dlrelu6(x: f32) -> f32 {
    if x < 0.0 {
        0.01
    } else if x <= 6.0 {
        1.0
    } else {
        0.01
    }
}

//const SCALE_FACTOR: f32 = 10.0;
//
//fn tanh(x: f32) -> f32 {
//    f32::tanh(x / SCALE_FACTOR)
//}
//
//fn dtanh(x: f32) -> f32 {
//    (tanh(x).mul_add(-tanh(x), 1.0)) / SCALE_FACTOR //sech^2(x)
//}

fn tanh(x: f32) -> f32 {
    f32::tanh(x)
}

fn dtanh(x: f32) -> f32 {
    f32::tanh(x).mul_add(-f32::tanh(x), 1.0) //sech^2(x)
}

pub fn safesoftmax(xs: &Vec<f32>) -> Vec<f32> {
    let mut total: f32 = 0.0;
    let mut output_vec: Vec<f32> = Vec::with_capacity(xs.len());
    let max_x = xs.iter().max_by(|&a, &b| a.total_cmp(&b)).unwrap();

    for &x in xs {
        let val: f32 = (x - max_x).exp();
        total += val;
        output_vec.push(val);
    }

    output_vec.iter().map(|x: &f32| x / total).collect()
}
