(import "stdlib.lisp")

(let path "soft-stdlib/mian/hello_world.lisp")

(defun mian/parse (sexpr)
  (match sexpr
    ([h1 `e] e)))

(defun mian/read-component (path)
  (let contents (read-file path))
  (let parsed (parse contents path))
  (let mian (list/map mian/parse parsed))
  mian)

(println (mian/read-component path))
