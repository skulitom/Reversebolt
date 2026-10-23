# Local text recognition models

Current previews use PaddlePaddle's unmodified **PP-OCRv6 medium** text detector
and recognizer, licensed Apache-2.0. Their exact upstream revisions, download
URLs, byte sizes and SHA-256 hashes are pinned in `manifest.json`.

- Project: https://github.com/PaddlePaddle/PaddleOCR
- Detector: https://huggingface.co/PaddlePaddle/PP-OCRv6_medium_det_onnx
- Recognizer: https://huggingface.co/PaddlePaddle/PP-OCRv6_medium_rec_onnx
- License copy: `licenses/PP-OCRv6-APACHE-2.0.txt` in the repository and preview.

The recognition dictionary is compiled into `reversebolt-vision`, with its
separate attribution and Apache-2.0 license in that crate. Reversebolt has not
fine-tuned the models. Original adapter code is MIT; RTen is MIT OR Apache-2.0.
This attribution does not imply endorsement by upstream authors.

Run `scripts/get-models.ps1` to obtain the pinned files (approximately 139 MB).
The desktop processes images locally and never downloads models itself. Missing,
partial or mismatched models leave Windows OCR available with a visible warning.
The preview publisher copies these notices, the manifest and model licenses.
An explicit MCP image-read request returns the selected image to that client.

## Earlier previews

Legacy Ocrs model installations remain supported when neither PP-OCRv6 model
directory is present. `ocrs-manifest.json` preserves their original hashes and
URLs. These unmodified weights are by Robert Knight and the Ocrs project,
licensed CC-BY-SA-4.0: https://huggingface.co/robertknight/ocrs and
https://github.com/robertknight/ocrs-models. Those legacy weights are not distributed
in this source release; retain their upstream license and attribution if
redistributing them. Current previews ship the PP-OCRv6 weights only. The two neural
versions never run as independent votes in the same observation.
