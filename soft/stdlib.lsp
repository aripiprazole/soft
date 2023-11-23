(declare defmacro* fun*)

(defmacro* map* (fun* [f coll]
  (if (cons? coll)
    (cons (f (car coll)) (map* f (cdr coll)))
    coll)))

(defmacro* quasi-quote (fun* [form]
  (if (cons? form)
    (if (= 'unquote (car form))
      (cdr form)
      (cons 'list (map* quasi-quote form)))
    (list 'quote form))))

(defmacro* defmacro (fun* [name args & body]
  (let [body `(begin ~body)]
    `(defmacro* ~name (fun* ~name ~args ~body)))))

(defmacro fun [args & body]
  (let [body `(begin ~body)]
    `(fun* local ~args ~body)))

(defmacro defun [name args & body]
  (let [body `(begin ~body)]
    `(def* ~name (fun* ~name ~args ~body))))
