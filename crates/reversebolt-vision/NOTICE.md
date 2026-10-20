# Attribution

Reversebolt's Rust image adapter, geometry and evaluation code are original MIT
code. The algorithms implement the general model interface documented by
[PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) and evaluated through
[RapidOCR](https://github.com/RapidAI/RapidOCR); no game profiles are supplied.

`assets/characters.txt` is the character dictionary extracted, without changes
to its entries or ordering, from PaddlePaddle's PP-OCRv6 medium recognition
export at revision `50c7eacafc52fa7bcf4194e8cd08e46f8558504b`,
`PostProcess.character_dict` in `inference.yml`. It is Apache-2.0, separately
from our source. The included `LICENSE-PaddleOCR.txt` retains that license.
The decoder adds the model's blank entry and space in memory.

The detector and recognizer weights are also Apache-2.0. They are loaded only
after size and SHA-256 verification and are not included in this crate.
Upstream revisions and hashes: `research/ocr-v6/models.json` at repository root.

Native rectangle expansion, interpolation and word geometry can differ slightly
from the OpenCV/Python reference. Native predictions require their own evaluation;
the earlier Python experiment is not a claim of native pipeline accuracy.
