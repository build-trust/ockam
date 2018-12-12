package test

import (
	"testing"
)

type mockFailNower struct {
	FailNowCalled bool
}

func (f *mockFailNower) FailNow() {
	f.FailNowCalled = true
}

func TestAssert(t *testing.T) {
	t.Run("succeeds when expected is equal to actual", func(t *testing.T) {
		Assert(t, true, true)
		Assert(t, []string{"a", "b"}, []string{"a", "b"})
		Assert(t, "aaa", "aaa")

		type bbbb struct {
			x int
			y int
		}

		type aaaa struct {
			a int
			b *bbbb
		}
		Assert(t, &aaaa{a: 100, b: &bbbb{x: 100, y: 200}}, &aaaa{a: 100, b: &bbbb{x: 100, y: 200}})
	})

	t.Run("calls t.FailNow when not equal and displays details of stdout", func(t *testing.T) {
		f := &mockFailNower{}

		stdout, stderr := Capture(func() {
			Assert(f, false, true)
		})

		Assert(t, true, f.FailNowCalled)
		Assert(t, "\n\tExpected: false\n\tActual: true\n", stdout[len(stdout)-32:])
		Assert(t, "", stderr)
	})

	t.Run("shows custom message id 4th arg is present", func(t *testing.T) {
		f := &mockFailNower{}
		stdout, _ := Capture(func() {
			Assert(f, false, true, "hello")
		})

		Assert(t, "\n\tExpected: false\n\tActual: true\n\thello\n\n", stdout[len(stdout)-40:])
	})

	t.Run("formats custom message if more that 4 args are passed", func(t *testing.T) {
		f := &mockFailNower{}
		stdout, _ := Capture(func() {
			Assert(f, false, true, "hello%shello%s", "a", "b")
		})

		Assert(t, "\n\tExpected: false\n\tActual: true\n\thelloahellob\n\n", stdout[len(stdout)-47:])
	})
}
