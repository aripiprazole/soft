(import "stdlib.lisp")

(defun foo (x)
  (throw "foo"))

(try* (foo 0)
  (err (err/print-stack err)))
