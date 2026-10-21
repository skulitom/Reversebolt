---
license: apache-2.0
tags:
  - vision
  - tracking
  - point-tracking
  - video
  - pytorch
  - jax
library_name: transformers
arxiv:
  - "2211.03726"
  - "2306.08637"
  - "2402.00847"
  - "2504.05579"
  - "2604.10582"
---

# Tracking Any Point (TAP) — Model Checkpoints

Official pre-trained checkpoints for the **Tracking Any Point (TAP)** family of models from Google DeepMind & Google Research.

[[GitHub](https://github.com/google-deepmind/tapnet)] [[TAP-Vid](https://tapvid.github.io/)] [[TAPIR](https://deepmind-tapir.github.io/)] [[RoboTAP](https://robotap.github.io/)] [[Blog Post](https://deepmind-tapir.github.io/blogpost.html)] [[BootsTAP](https://bootstap.github.io/)] [[TAPVid-3D](https://tapvid3d.github.io/)] [[TAPNext](https://tap-next.github.io/)] [[TRAJAN](https://trajan-paper.github.io)] [[TAPNext++](https://tap-next-plus-plus.github.io/)]

## Model Overview

The TAP model family addresses the task of **tracking any point on any solid physical surface** across video frames. Given a query point on one frame, the model predicts where that point moves to on every other frame, along with an occlusion/visibility estimate.

| Model | Key Contribution |
|:--|:--|
| **TAP-Net** | Baseline model introduced with the TAP-Vid benchmark |
| **TAPIR** | Two-stage matching + refinement; large accuracy gains over TAP-Net |
| **BootsTAPIR** | TAPIR + bootstrapped self-training on unlabeled video |
| **TAPNext / BootsTAPNext** | Next-token-prediction formulation; fastest and simplest tracker |
| **TAPNext++** | Fine-tuned BootsTAPNext with 40× longer stable tracking, occlusion handling, and re-detection |
| **TRAJAN** | Point trajectory autoencoder for motion representation learning |

## Checkpoints

All checkpoints are released under the **Apache License 2.0**.

### TAPNext++ (latest)

| File | Resolution | Framework | DAVIS First (AJ) | Kinetics First (AJ) | RoboTAP First (AJ) |
|:--|:--:|:--:|:--:|:--:|:--:|
| `tapnextpp_ckpt.pt` | 256×256 | PyTorch | 65.6% | 53.9% | 61.1% |
| `tapnextpp_512.ckpt` | 512×512 | PyTorch | 67.0% | — | — |

### TAPNext / BootsTAPNext

| File | Resolution | Framework | DAVIS First (AJ) |
|:--|:--:|:--:|:--:|
| `bootstapnext.pth` | 256×256 | PyTorch | 65.25% |
| `bootstapnext_ckpt.npz` | 256×256 | Jax | 65.25% |

### TAPIR & BootsTAPIR

| File | Mode | Resolution | Framework | DAVIS First (AJ) |
|:--|:--:|:--:|:--:|:--:|
| `tapir_checkpoint_panning.pt` | Offline | 256×256 | PyTorch | 58.5% |
| `tapir_checkpoint_panning.npy` | Offline | 256×256 | Jax | 58.5% |
| `causal_tapir_checkpoint.npy` | Online | 256×256 | Jax | 56.2% |
| `bootstapir_checkpoint_v2.pt` | Offline | 256×256 + 512×512 | PyTorch | 62.4% |
| `bootstapir_checkpoint_v2.npy` | Offline | 256×256 + 512×512 | Jax | 62.4% |
| `causal_bootstapir_checkpoint.pt` | Online | 256×256 + 512×512 | PyTorch | 59.7% |
| `causal_bootstapir_checkpoint.npy` | Online | 256×256 + 512×512 | Jax | 59.7% |

### Other

| File | Model | Framework | DAVIS First (AJ) |
|:--|:--|:--:|:--:|
| `checkpoint.npy` | TAP-Net (baseline) | Jax | 33.0% |
| `track_autoencoder_ckpt.npz` | TRAJAN | Jax | — |

## Quick Start

### TAPNext++ (PyTorch)

```python
import torch
from tapnet.tapnextpp.votsp2026.model import TAPNextPP

# Load model (handles checkpoint parsing internally)
model = TAPNextPP.from_checkpoint(
    "tapnextpp_ckpt.pt",       # or "tapnextpp_512.ckpt" for 512×512
    device="cuda",
    input_resolution=256,      # use 512 for the 512×512 checkpoint
)

# Online tracking: first frame with query points
positions, visible, state = model.track_frame(frame_bgr, query_points_xy=query_xy)

# Subsequent frames
for frame in subsequent_frames:
    positions, visible, state = model.track_frame(frame, state=state)
```

See the [TAPNext++ Colab](https://colab.research.google.com/github/deepmind/tapnet/blob/main/colabs/torch_tapnextpp_demo.ipynb) for a complete demo.

### BootsTAPIR (PyTorch)

```python
import torch
from tapnet.torch import tapir_model

# Load model
model = tapir_model.TAPIR(pyramid_level=1)
model.load_state_dict(torch.load("bootstapir_checkpoint_v2.pt"))
model = model.to(device)
model.eval()
```

See the [PyTorch TAPIR Colab](https://colab.research.google.com/github/deepmind/tapnet/blob/master/colabs/torch_tapir_demo.ipynb) for a complete demo.

## Installation

```bash
pip install "tapnet[torch] @ git+https://github.com/google-deepmind/tapnet.git"
```

## Citation

If you use these models, please cite the relevant papers:

<details>
<summary>TAP-Vid (NeurIPS 2022)</summary>

```bibtex
@article{doersch2022tap,
  title={{TAP}-Vid: A Benchmark for Tracking Any Point in a Video},
  author={Doersch, Carl and Gupta, Ankush and Markeeva, Larisa and Recasens, Adria and Smaira, Lucas and Aytar, Yusuf and Carreira, Joao and Zisserman, Andrew and Yang, Yi},
  journal={Advances in Neural Information Processing Systems},
  volume={35},
  pages={13610--13626},
  year={2022}
}
```
</details>

<details>
<summary>TAPIR (ICCV 2023)</summary>

```bibtex
@inproceedings{doersch2023tapir,
  title={{TAPIR}: Tracking any point with per-frame initialization and temporal refinement},
  author={Doersch, Carl and Yang, Yi and Vecerik, Mel and Gokay, Dilara and Gupta, Ankush and Aytar, Yusuf and Carreira, Joao and Zisserman, Andrew},
  booktitle={Proceedings of the IEEE/CVF International Conference on Computer Vision},
  pages={10061--10072},
  year={2023}
}
```
</details>

<details>
<summary>RoboTAP (ICRA 2024)</summary>

```bibtex
@article{vecerik2023robotap,
  title={{RoboTAP}: Tracking arbitrary points for few-shot visual imitation},
  author={Vecerik, Mel and Doersch, Carl and Yang, Yi and Davchev, Todor and Aytar, Yusuf and Zhou, Guangyao and Hadsell, Raia and Agapito, Lourdes and Scholz, Jon},
  journal={International Conference on Robotics and Automation},
  pages={5397--5403},
  year={2024}
}
```
</details>

<details>
<summary>BootsTAP (ACCV 2024)</summary>

```bibtex
@article{doersch2024bootstap,
  title={{BootsTAP}: Bootstrapped Training for Tracking-Any-Point},
  author={Doersch, Carl and Luc, Pauline and Yang, Yi and Gokay, Dilara and Koppula, Skanda and Gupta, Ankush and Heyward, Joseph and Rocco, Ignacio and Goroshin, Ross and Carreira, Jo{\~a}o and Zisserman, Andrew},
  journal={Asian Conference on Computer Vision},
  year={2024}
}
```
</details>

<details>
<summary>TAPVid-3D (NeurIPS 2024)</summary>

```bibtex
@article{koppula2024tapvid,
  title={{TAPVid}-{3D}: A Benchmark for Tracking Any Point in {3D}},
  author={Koppula, Skanda and Rocco, Ignacio and Yang, Yi and Heyward, Joe and Carreira, Jo{\~a}o and Zisserman, Andrew and Brostow, Gabriel and Doersch, Carl},
  journal={Advances in Neural Information Processing Systems},
  year={2024}
}
```
</details>

<details>
<summary>TAPNext (ICCV 2025)</summary>

```bibtex
@inproceedings{zholus2025tapnext,
  title={Tapnext: Tracking any point (tap) as next token prediction},
  author={Zholus, Artem and Doersch, Carl and Yang, Yi and Koppula, Skanda and Patraucean, Viorica and He, Xu Owen and Rocco, Ignacio and Sajjadi, Mehdi SM and Chandar, Sarath and Goroshin, Ross},
  booktitle={Proceedings of the IEEE/CVF International Conference on Computer Vision},
  pages={9693--9703},
  year={2025}
}
```
</details>

<details>
<summary>TRAJAN (2025)</summary>

```bibtex
@article{allen2025trajan,
  title={Direct Motion Models for Assessing Generated Videos},
  author={Allen, Kelsey and Doersch, Carl and Zhou, Guangyao and Suhail, Mohammed and Driess, Danny and Rocco, Ignacio and Rubanova, Yulia and Kipf, Thomas and Sajjadi, Mehdi S. M. and Murphy, Kevin and Carreira, Joao and van Steenkiste, Sjoerd},
  journal={arXiv preprint},
  year={2025}
}
```
</details>

<details>
<summary>TAPNext++ (CVPR 2026 Findings)</summary>

```bibtex
@InProceedings{Jung_2026_CVPR,
  author    = {Jung, Sebastian and Zholus, Artem and Sundermeyer, Martin and Doersch, Carl
               and Goroshin, Ross and Tan, David Joseph and Chandar, Sarath
               and Triebel, Rudolph and Tombari, Federico},
  title     = {TAPNext++: What's Next for Tracking Any Point (TAP)?},
  booktitle = {Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR) Findings},
  month     = {June},
  year      = {2026},
  pages     = {8429-8438}
}
```
</details>

## License

Copyright 2022-2026 Google LLC

All software and model checkpoints in this repository are licensed under the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

This is not an official Google product.
