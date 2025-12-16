# -*- coding: utf-8 -*-
"""
移除聲調相關函數
"""

import re


def remove_tone(text: str) -> str:
    """
    移除數字聲調版本中的聲調數字和連字符

    Args:
        text: 數字聲調版本的羅馬字 (如 gua2 或 gua2ho2)

    Returns:
        無聲調版本，或空字串（輸入為空時）
    """
    if not text:
        return ""

    # 移除數字 (1-9) 和連字符
    return re.sub(r"[\d\-]", "", text)
