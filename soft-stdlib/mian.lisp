(import "stdlib.lisp")

(defstruct analysis-result 
            :variables 
            :changes
            :uses-in-template)

; The result of parsing a component
(defstruct parse-result
            :style
            :html
            :script)

; Transforms an s-expr into a rich data structure that can be used for analysis and traversal. It
; always returns a parse-result
(defun mian/parse (expr)
  (let result (make-parse-result))

  (letfun parse-attribute (attr)
    (match attr
      (((atom id) `value) ~(:attribute `id `value))
      (_                   (throw "invalid attribute"))))

  (letfun parse-html (expr)
    (match expr
      ((id id)
        ~(:var `id))
      ((str str) 
        ~(:text `str))
      ((`tag (list &attrs) &body) 
        ~(`(to-atom tag) `(map* parse-attribute (tail attrs)) `(map* parse-html body)))
      (_
        (throw "invalid html"))))

  ; Parses a top-level statement in a component
  (letfun parse-top-level (expr)
    (match expr
      ((script &code)
        (map/set! result :script code))
      ((style &code)
        (map/set! result :style code))
      ((`other &code) 
        (let vector (map/get result :html))
        (if (nil? vector)
          (map/set! result :html (vec (parse-html expr)))
          (vec/push! vector (parse-html expr))))))

  (map* parse-top-level expr)

  result)

; Checks if variables are used in templates, if they chance and what are the variables in order to
; generate optimized code.
(defun mian/analysis (script html) 
  (let result (make-analysis-result (hash-set) (hash-set) (hash-set)))

  (letfun analyze-expr (on-global variables expr)
    (match expr
      ((let `name `expr) 
        (if on-global
          (block
            (map/set! variables name :true)
            (analyze-expr on-global variables expr))
          (block
            (map/set! variables name ())
            (analyze-expr on-global variables expr))))
      ((fn (&rest) &body)
        (let vars (clone variables))
        (list/each (lambda (x) (analyze-expr () vars x)) body))
      ((set (id name) `expr)
        (when (and (not on-global) (= :true (map/get variables name)))
              (map/set! (map/get result :changes) name :true)))
      ((+ &rest)
        (list/each (lambda (x) (analyze-expr on-global variables x)) expr))
      ((- &rest)
        (list/each (lambda (x) (analyze-expr on-global variables x)) expr))
      (_ expr)))

  (list/each (lambda (x) (analyze-expr :true (map/get result :variables) x)) script)
  
  (letfun analyze-html (expr)
    (match expr
      ((:attribute `tag `body)
        (list/each analyze-html body))
      (((atom tag) (&attrs) &body)
        (list/each analyze-html body)
        (list/each analyze-html attrs))
      (((:var `id))
        (map/set! (map/get result :uses-in-template) id :true))
      ((id id) 
        (map/set! (map/get result :uses-in-template) id :true))
      (((:text `str)) ())))

  (vec/each analyze-html html)

  result)

(defun mian/compile (analysis parsed) 
  (let counter (hash-map (:counter 0)))
  
  (letfun inc ()
    (map/update! counter :counter (lambda (x) (+ x 1)))
    (map/get counter :counter))

  (letfun compile-html (html-vec script)
    (let variables (vec))
    (let create (vec))
    (let destroy (vec))
    (let update (vec))

    (let result (hash-map 
      (:variables variables) 
      (:create create) 
      (:destroy destroy) 
      (:update update)))

    (letfun compile-html (html parent)
      (match html
        ((:attribute :on-click `body) 
          (vec/push! create (string/concat "listen(" parent ", 'click', " (to-string body) ")")))
        ((:var `var)
          (let name (string/concat "text_" (to-string (inc))))
          (vec/push! variables name)
          (vec/push! create (string/concat name " = text(" parent ", " (to-string var) ")"))
          (vec/push! destroy (string/concat "destroy(" parent ", " name ")"))
          (let used (map/get analysis :uses-in-template))
          (when (= :true (map/get used var))
                (vec/push! update 
                  (string/concat  
                    "if (changed.includes('" (to-string var) "')) {" 
                          name ".data = " (to-string var) 
                    "}"))))
        ((:text `text)
          (let name (string/concat "text_" (to-string (inc))))
          (vec/push! variables name)
          (vec/push! create (string/concat name " = text(" parent ",'" text "')"))
          (vec/push! destroy (string/concat "destroy(" parent ", " name ")")))
        ((`tag (&attrs) `body)
          (let name (string/concat (to-string tag) "_" (to-string (inc))))
          (vec/push! variables name)
          (vec/push! create (string/concat name " = create(" parent ", '" (to-string tag) "')"))
          (vec/push! destroy (string/concat "destroy(" parent ", " name ")"))
          (list/each (lambda (x) (compile-html x name)) attrs)
          (list/each (lambda (x) (compile-html x name)) body))))

    (letfun aggregate (result script)
      (string/concat
        "function main() {"
            "let " (string/join (list/intersperse ", " (vec/to-list(map/get result :variables)))) ";"
            "let comp = {"
              "c(target) {"
                  (string/join (list/intersperse ";" (vec/to-list(map/get result :create))))
              "},"
              "u(changed) {"
                  (string/join (list/intersperse ";" (vec/to-list(map/get result :update))))
              "},"
              "d() {"
                  (string/join (list/intersperse ";" (vec/to-list(map/get result :destroy))))
              "}"
            "};"
            (string/join script)
            "return comp;"
        "}"))

    (vec/each (lambda (x) (compile-html x "target")) html-vec)
  
    (aggregate result script))

  (letfun compile-js (js)
     (match js     
        ((let `name `expr)
          (string/concat "let " (to-string name) " = " (compile-js expr) ";"))
        ((fn (&rest) &body)
          (string/concat "(" (string/join (list/intersperse "," rest)) ") => {" 
                             (string/join (list/intersperse ";" (list/map compile-js body))) "}"))
        ((set (id name) `expr)
          (let expr (string/concat (to-string name) " = " (compile-js expr)))
          (if (map/get (map/get analysis :changes) name)
            (string/concat "(" expr ", comp.u(['" (to-string name) "']))")
            (string/concat expr ";")))
        ((+ &rest)
          (string/join (list/intersperse " + " (list/map compile-js rest))))
        ((- &rest)
          (string/join (list/intersperse " - " (list/map compile-js rest))))
        ((int x) (to-string x))
        ((id x) (to-string x))))


  (let script (map* compile-js (map/get parsed :script)))
  (let result (compile-html (map/get parsed :html) script))
  result)

; Reads a component from a file and then analyses it 
(defun mian/read-component (path)
  (let contents    (read-file path))
  (let parsed      (parse contents path))
  (let specialized (mian/parse parsed))
  (let analysis    (mian/analysis (map/get specialized :script) (map/get specialized :html)))
  (let compiled    (mian/compile analysis specialized))
  compiled)
