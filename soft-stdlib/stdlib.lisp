(import "macros.lisp")
(import "list.lisp")

(defun map/pairs (map)
  (list/zip (map/keys map) (map/vals map)))

(defun pair (a b) (list a b))

(defun fst (pair) (head pair))

(defun snd (pair) (head (tail pair)))

(defun not (x) (if x () :true))

(defun atomic? (x) (not (list/is? x)))

(defmacro hash-set (&rest rest) (cons 'hash-map (map* (lambda (x) (pair x x)) rest)))

(defun hash-set/insert (set key) (map/insert set key key))

(defun map/contains? (set key) (not (nil? (map/get set key))))

(defun map/update! (map key f) (map/set! map key (f (map/get map key))))

(defmacro defstruct (name &rest fields)
  (let name (to-id (string/concat "make-"(to-string name))))
  (let params (map* to-id fields))
  ~(defun `name `(list/join (map* (lambda (x) ~(&optional `(to-id x))) params)) `(cons 'hash-map (list/zip fields params))))

(defun vec/each (f vec)
  (let i 0)
  (while (< (clone i) (vec/len vec))
    (block
      (f (vec/get vec (clone i)))
      (set i (+ i 1)))))

(defun vec/map (f vector)
  (let result (vec))
  (vec/each (lambda (x) (vec/push! result (f x))) vector)
  result)

(defun vec/reduce (f init vector)
  (let result init)
  (let i 0)
  (while (< (clone i) (vec/len vector))
    (block
      (set result (f result (vec/get vector (clone i))))
      (set i (+ i 1))))
  result)

(defun list/reverse (lis)
  (if (cons? lis)
      (list/concat (list/reverse (tail lis)) (list (head lis)))
      ()))

(defun vec/to-list (vector)
  (list/reverse (vec/reduce (lambda (acc x) (cons x acc)) () vector)))
