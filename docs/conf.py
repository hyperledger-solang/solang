# SPDX-License-Identifier: Apache-2.0
#
# Configuration file for the Sphinx documentation builder.
#
# This file only contains a selection of the most common options. For a full
# list see the documentation:
# http://www.sphinx-doc.org/en/master/config

# -- Path setup --------------------------------------------------------------

# If extensions (or modules to document with autodoc) are in another directory,
# add these directories to sys.path here. If the directory is relative to the
# documentation root, use os.path.abspath to make it absolute, like shown here.
#
# import os
# import sys
# sys.path.insert(0, os.path.abspath('.'))
import os

from pygments_lexer_solidity import SolidityLexer, YulLexer

def setup(sphinx):
    sphinx.add_lexer('Solidity', SolidityLexer)
    sphinx.add_lexer('Yul', YulLexer)

# -- Project information -----------------------------------------------------

project = 'Solang Solidity Compiler'
copyright = '2019 - 2023 Solang Maintainers'
author = 'Sean Young <sean@mess.org>, Cyrill Leutwiler <bigcyrill@hotmail.com>, Lucas Steuernagel <lucas.tnagel@gmail.com>'

# The full version, including alpha/beta/rc tags
version = os.popen('git describe --tags --abbrev=0').readline().strip()
release = os.popen('git describe --tags').readline().strip()

# -- General configuration ---------------------------------------------------

# Add any Sphinx extension module names here, as strings. They can be
# extensions coming with Sphinx (named 'sphinx.ext.*') or your custom
# ones.
extensions = [
    'sphinx_tabs.tabs'
]

# Do not allow tabs to be closed
sphinx_tabs_disable_tab_closing = True
# Add any paths that contain templates here, relative to this directory.
templates_path = ['_templates']

# List of patterns, relative to source directory, that match files and
# directories to ignore when looking for source files.
# This pattern also affects html_static_path and html_extra_path.
exclude_patterns = []


# -- Options for HTML output -------------------------------------------------

# The theme to use for HTML and HTML Help pages.  See the documentation for
# a list of builtin themes.
#
#html_theme = 'alabaster'
html_theme = 'sphinx_rtd_theme'

# Add any paths that contain custom static files (such as style sheets) here,
# relative to this directory. They are copied after the builtin static files,
# so a file named "default.css" will overwrite the builtin "default.css".
# html_static_path = ['_static']

# See https://github.com/readthedocs/readthedocs.org/issues/2569
master_doc = 'index'
