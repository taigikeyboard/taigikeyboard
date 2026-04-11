# -*- coding: utf-8 -*-
"""
共用 logging 設定
"""

import os
import logging
from datetime import datetime


def setup_logging(script_name: str, log_dir: str = "logs") -> logging.Logger:
    """
    設定 logging，同時輸出到檔案和螢幕

    Args:
        script_name: 腳本名稱（用於 log 檔名）
        log_dir: log 檔案目錄

    Returns:
        Logger 實例
    """
    os.makedirs(log_dir, exist_ok=True)
    log_file = os.path.join(log_dir, f"{script_name}.log")

    # 清除既有的 handlers（避免重複設定）
    root_logger = logging.getLogger()
    root_logger.handlers.clear()

    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",
        handlers=[
            logging.FileHandler(log_file, mode="w", encoding="utf-8"),
            logging.StreamHandler(),
        ],
    )
    return logging.getLogger(__name__)


def log_header(logger: logging.Logger, script_name: str, input_path: str, output_path: str):
    """
    輸出標準 header

    Args:
        logger: Logger 實例
        script_name: 腳本名稱
        input_path: 輸入路徑
        output_path: 輸出路徑
    """
    logger.info(f"{'=' * 50}")
    logger.info(f"{script_name}")
    logger.info(f"Run: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    logger.info(f"{'=' * 50}")
    logger.info(f"Input:  {input_path}")
    logger.info(f"Output: {output_path}")
    logger.info("")
