(import "macros.lisp")
(import "list.lisp")


(defun pair (a b) (list a b))

(defun fst (pair) (head pair))

(defun snd (pair) (head (tail pair)))

(defun list/ref (list index) 
  (if (= index 0) (head list) (list/ref (tail list) (- index 1))))

(defun list/is? (list) (if (= (type-of list) :cons) :true ()))

(defun list/zip (a b)
  (if (or (nil? a) (nil? b)) 
    ()
    (cons (pair (head a) (head b)) (list/zip (tail a) (tail b)))))

(defun hash-map/pairs (map)
  (list/zip (hash-map/keys map) (hash-map/vals map)))

(defun not (x) (if x () :true))

(defun atomic? (x) (not (list/is? x)))

(defun expand-rule (scrutinee pattern bindings) 
  (letfun expand-rule/cons (scrutinee start end bindings)
    (cond start
      ('unquote (block
          (hash-map/set! bindings (snd pattern) scrutinee)
          ~(atomic? `scrutinee)))
      ('list* (block 
        (hash-map/set! bindings (head end) scrutinee)
        ~(list/is? `scrutinee)))
      ('list (block 
        (let conditions 
          (list ~(list/is? `scrutinee) 
                ~(= `(list/length end) (list/length `scrutinee))))
        (let expanded 
            (list/map 
              (lambda (x) (expand-rule 
                            ~(list/ref `scrutinee `(fst x)) 
                            (snd x) 
                            bindings))
              (list/enumerate end)))
        (let conditions (list/concat conditions expanded))
        (list/fold conditions :true (lambda (x y) (if (= y :true) x (if (= x :true) y ~(and `x `y)))))))))

  (cond (type-of pattern)
    (:cons (expand-rule/cons scrutinee (head pattern) (tail pattern) bindings))
    (else   ~(= `scrutinee (quote `pattern)))))
        
(defun match/rule (pattern then)
  (let bindings (hash-map))
  (let expanded (expand-rule '#cond pattern bindings))
  
  (let then_block (list/map (lambda (x) ~(let `(fst x) `(snd x))) (hash-map/pairs bindings)))
  (let then_block (list/concat then_block (list then)))
  
  ~(`expanded `(cons 'block then_block)))

(defmacro match (scrutinee &rest cases)
  (let cases (list/map (lambda (x) (match/rule (fst x) (snd x))) cases))
  ~(block 
      (let #cond `scrutinee)
      `(cons 'case (list/concat cases '(('else (throw "match: no match")))))))

(match (list 'x (list 10 20))
  ((list x (list* y)) (print y))
  (2 (print "ata")))