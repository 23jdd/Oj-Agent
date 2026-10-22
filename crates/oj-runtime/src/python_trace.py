import sys, json, io, traceback

request = json.loads(sys.stdin.read(200001))
source, sample = request['source'], request.get('values', [])
events, last_values = [], list(sample)
limit = min(10000, max(1, request.get('max_events', 1000)))

class TraceLimit(BaseException): pass
class OutputLimit(BaseException): pass
class BoundedOutput(io.StringIO):
    def write(self, value):
        if self.tell() + len(value) > 1048576: raise OutputLimit('stdout exceeded 1 MiB')
        return super().write(value)

def serializable(value):
    if value is None or type(value) in (bool, int, float): return value
    if type(value) is str: return value[:256]
    if type(value) in (list, tuple) and len(value) <= 64 and all(type(x) in (int, bool) for x in value): return list(value)
    if type(value) is dict and len(value) <= 32 and all(type(k) in (str, int) and type(v) in (int, bool, str) for k, v in value.items()): return {str(k):v for k,v in value.items()}
    return None

def observe(frame, event, arg):
    global last_values
    if frame.f_code.co_filename != 'solution.py' or event not in ('line', 'return'): return observe
    if len(events) >= limit: raise TraceLimit('trace event limit exceeded')
    state = {k:serializable(v) for k,v in list(frame.f_locals.items())[:32] if not k.startswith('__') and serializable(v) is not None}
    arrays = [(k,v) for k,v in state.items() if type(v) is list and v and all(type(n) is int for n in v)]
    if arrays:
        chosen = next((v for k,v in arrays if k in ('values','nums','arr','array','a','dp')), arrays[0][1])
        last_values = chosen
    indices = [v for k,v in state.items() if k in ('i','j','mid','left','right','index') and type(v) is int and 0 <= v < len(last_values)]
    if event == 'return': state['return_value'] = serializable(arg)
    seq = len(events)
    events.append(dict(id=f'event_{seq}', seq=seq, kind='return' if event=='return' else 'read', entity='values', indices=indices, values=list(last_values), explanation=f'{frame.f_code.co_name}: line {frame.f_lineno}', state=state, source=dict(file='solution.py', line=frame.f_lineno, function=frame.f_code.co_name)))
    return observe

output = BoundedOutput()
previous_out, previous_in = sys.stdout, sys.stdin
try:
    sys.stdout, sys.stdin = output, io.StringIO(request.get('stdin',''))
    sys.settrace(observe)
    exec(compile(source, 'solution.py', 'exec'), {'__name__':'__main__'})
    sys.settrace(None)
except BaseException:
    sys.settrace(None)
    sys.stdout, sys.stdin = previous_out, previous_in
    traceback.print_exc()
    sys.exit(1)
finally:
    sys.settrace(None)
    sys.stdout, sys.stdin = previous_out, previous_in

print(json.dumps(dict(stdout=output.getvalue(), events=events), allow_nan=False))
