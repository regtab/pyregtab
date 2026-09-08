"""``python -m pyregtab <pattern.rtl>`` — alias of ``python -m pyregtab.runner``."""

import sys

from pyregtab.runner import main

sys.exit(main())
