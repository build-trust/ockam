defmodule Makeup.Styles.HTML.StyleMap do
  @moduledoc """
  This module contains all styles, and facilities to map style names (binaries or atoms) to styles.

  Style names are of the form `<name>_style`.
  """

  alias Makeup.Styles.HTML

  # %% Start Pygments %%

  @doc """
  The *abap* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#abap).
  """
  def abap_style, do: HTML.AbapStyle.style()

  @doc """
  The *algol* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#algol).
  """
  def algol_style, do: HTML.AlgolStyle.style()

  @doc """
  The *algol_nu* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#algol_nu).
  """
  def algol_nu_style, do: HTML.Algol_NuStyle.style()

  @doc """
  The *arduino* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#arduino).
  """
  def arduino_style, do: HTML.ArduinoStyle.style()

  @doc """
  The *autumn* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#autumn).
  """
  def autumn_style, do: HTML.AutumnStyle.style()

  @doc """
  The *borland* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#borland).
  """
  def borland_style, do: HTML.BorlandStyle.style()

  @doc """
  The *bw* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#bw).
  """
  def bw_style, do: HTML.BlackWhiteStyle.style()

  @doc """
  The *colorful* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#colorful).
  """
  def colorful_style, do: HTML.ColorfulStyle.style()

  @doc """
  The *default* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#default).
  """
  def default_style, do: HTML.DefaultStyle.style()

  @doc """
  The *emacs* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#emacs).
  """
  def emacs_style, do: HTML.EmacsStyle.style()

  @doc """
  The *friendly* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#friendly).
  """
  def friendly_style, do: HTML.FriendlyStyle.style()

  @doc """
  The *fruity* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#fruity).
  """
  def fruity_style, do: HTML.FruityStyle.style()

  @doc """
  The *igor* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#igor).
  """
  def igor_style, do: HTML.IgorStyle.style()

  @doc """
  The *lovelace* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#lovelace).
  """
  def lovelace_style, do: HTML.LovelaceStyle.style()

  @doc """
  The *manni* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#manni).
  """
  def manni_style, do: HTML.ManniStyle.style()

  @doc """
  The *monokai* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#monokai).
  """
  def monokai_style, do: HTML.MonokaiStyle.style()

  @doc """
  The *murphy* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#murphy).
  """
  def murphy_style, do: HTML.MurphyStyle.style()

  @doc """
  The *native* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#native).
  """
  def native_style, do: HTML.NativeStyle.style()

  @doc """
  The *paraiso_dark* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#paraiso_dark).
  """
  def paraiso_dark_style, do: HTML.ParaisoDarkStyle.style()

  @doc """
  The *paraiso_light* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#paraiso_light).
  """
  def paraiso_light_style, do: HTML.ParaisoLightStyle.style()

  @doc """
  The *pastie* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#pastie).
  """
  def pastie_style, do: HTML.PastieStyle.style()

  @doc """
  The *perldoc* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#perldoc).
  """
  def perldoc_style, do: HTML.PerldocStyle.style()

  @doc """
  The *rainbow_dash* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#rainbow_dash).
  """
  def rainbow_dash_style, do: HTML.RainbowDashStyle.style()

  @doc """
  The *rrt* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#rrt).
  """
  def rrt_style, do: HTML.RrtStyle.style()

  @doc """
  The *tango* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#tango).
  """
  def tango_style, do: HTML.TangoStyle.style()

  @doc """
  The *trac* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#trac).
  """
  def trac_style, do: HTML.TracStyle.style()

  @doc """
  The *vim* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#vim).
  """
  def vim_style, do: HTML.VimStyle.style()

  @doc """
  The *vs* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#vs).
  """
  def vs_style, do: HTML.VisualStudioStyle.style()

  @doc """
  The *xcode* style. Example [here](https://tmbb.github.io/makeup_demo/elixir.html#xcode).
  """
  def xcode_style, do: HTML.XcodeStyle.style()

  # %% End Pygments %%

  # Custom themes:
  @doc """
  The *samba* style, based on the tango style, but with visual distinction between
  classes and variables, and lighter punctuation.
  """
  def samba_style, do: HTML.SambaStyle.style()
end
