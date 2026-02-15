type Operation = (effect: SimpleEffect<any>) => SimpleEffect<any>

class SimpleEffect<A> {
  constructor(private readonly thunk: () => Promise<A>) {}

  run(): Promise<A> {
    return this.thunk()
  }

  pipe(...operations: Operation[]): SimpleEffect<any> {
    return operations.reduce((current, operation) => operation(current), this)
  }

  [Symbol.iterator](): Iterator<SimpleEffect<A>, A, A> {
    let emitted = false
    const self = this

    return {
      next(input?: A): IteratorResult<SimpleEffect<A>, A> {
        if (!emitted) {
          emitted = true
          return { done: false, value: self }
        }

        return { done: true, value: input as A }
      },
    }
  }
}

function make<A>(thunk: () => Promise<A>): SimpleEffect<A> {
  return new SimpleEffect(thunk)
}

export const Effect = {
  succeed<A>(value: A): SimpleEffect<A> {
    return make(() => Promise.resolve(value))
  },

  try<A>(options: { try: () => A; catch: (error: unknown) => A }): SimpleEffect<A> {
    return make(async () => {
      try {
        return options.try()
      } catch (error) {
        return options.catch(error)
      }
    })
  },

  tryPromise<A>(options: {
    try: () => Promise<A> | A
    catch: (error: unknown) => unknown
  }): SimpleEffect<A> {
    return make(async () => {
      try {
        return await options.try()
      } catch (error) {
        throw options.catch(error)
      }
    })
  },

  map<A, B>(mapper: (value: A) => B) {
    return (effect: SimpleEffect<A>): SimpleEffect<B> =>
      make(async () => mapper(await effect.run()))
  },

  catchAll<A>(handler: (error: unknown) => SimpleEffect<A>) {
    return (effect: SimpleEffect<A>): SimpleEffect<A> =>
      make(async () => {
        try {
          return await effect.run()
        } catch (error) {
          return handler(error).run()
        }
      })
  },

  gen<A>(factory: () => Generator<SimpleEffect<any>, A, any>): SimpleEffect<A> {
    return make(async () => {
      const iterator = factory()
      let state = iterator.next()

      while (!state.done) {
        try {
          const value = await state.value.run()
          state = iterator.next(value)
        } catch (error) {
          if (!iterator.throw) {
            throw error
          }
          state = iterator.throw(error)
        }
      }

      return state.value
    })
  },

  runPromise<A>(effect: SimpleEffect<A>): Promise<A> {
    return effect.run()
  },
}
