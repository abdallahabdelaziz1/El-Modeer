import './App.css';

import { ColumnDirective, ColumnsDirective, TreeGridComponent } from '@syncfusion/ej2-react-treegrid';
import { Resize, ColumnMenu, Filter, Inject, Sort } from '@syncfusion/ej2-react-treegrid';
import * as React from 'react';
import { registerLicense } from '@syncfusion/ej2-base';
import { useEffect, useState } from 'react';


import { invoke } from '@tauri-apps/api'

// Registering Syncfusion license key
registerLicense("Mgo+DSMBaFt+QHFqVkNrXVNbdV5dVGpAd0N3RGlcdlR1fUUmHVdTRHRcQlliTH5WdUBjXXdZcXQ=;Mgo+DSMBPh8sVXJ1S0d+X1RPd11dXmJWd1p/THNYflR1fV9DaUwxOX1dQl9gSXpSc0VmWnpedHxdQGA=;ORg4AjUWIQA/Gnt2VFhhQlJBfV5AQmBIYVp/TGpJfl96cVxMZVVBJAtUQF1hSn5Xd0BjXHxbcHBcRmhc;MTczMjcyNUAzMjMxMmUzMTJlMzMzNWs5MUM5SzZnOEg2dlFtSEpvaDY4WE9aZjVUdDhVdmRkN1ZTVUlVZC9TVlE9;MTczMjcyNkAzMjMxMmUzMTJlMzMzNWZGLzNtVEdQZ011S1Jkd2JxcW0wSkE1UXdIaTJ4UWVCQ3VzaU51dWRDekk9;NRAiBiAaIQQuGjN/V0d+XU9Hc1RDX3xKf0x/TGpQb19xflBPallYVBYiSV9jS31TckRmWXpddXRQRmBYUQ==;MTczMjcyOEAzMjMxMmUzMTJlMzMzNWI2MVp4ZDc5am8rUnZ3TXNicVArYmtEMVNYU3lvczI4Nmg3VkxYaVFCNDA9;MTczMjcyOUAzMjMxMmUzMTJlMzMzNWg3Q0trVkZqZnJSaUo3YTZmR2dQQnh3aVhVdC9TZlh4U0VheE1MWVRKalk9;Mgo+DSMBMAY9C3t2VFhhQlJBfV5AQmBIYVp/TGpJfl96cVxMZVVBJAtUQF1hSn5Xd0BjXHxbcHFVTmVc;MTczMjczMUAzMjMxMmUzMTJlMzMzNWJ4RmY3WFFpR1VTNXJRSzNDMXE1dEdBdXVQb2orYjVxY3djVDZCWVV3RWs9;MTczMjczMkAzMjMxMmUzMTJlMzMzNUpSd29mTHZmUldxNjBkNXdMamlCMm0veWJ3VlRSdWU4RW5mODk1WThsOGs9;MTczMjczM0AzMjMxMmUzMTJlMzMzNWI2MVp4ZDc5am8rUnZ3TXNicVArYmtEMVNYU3lvczI4Nmg3VkxYaVFCNDA9");

function App() {

    const MINUTE_MS = 10000;

    const [processes, setProcesses] = useState([]);

    useEffect(() => {

        invoke('get_processes').then((message) => setProcesses(JSON.parse(message)["children"]))

        const interval = setInterval(() => {
            invoke('get_processes').then((message) => setProcesses(JSON.parse(message)["children"]))
        }, MINUTE_MS);

        return () => clearInterval(interval);
    }, [])

    let treegrid;
    const dataBound = () => {
            treegrid.autoFitColumns(['name', 'pid', 'ppid', 'state', 'vmsize', 'nice', 'cpu_time']);
    };

    const filterSettings = { 
        type: 'Menu', 
        hierarchyMode: 'Both'
    };
    
    return <TreeGridComponent 
        height='530' width='785'
        dataBound={dataBound} ref={g => treegrid = g}
        dataSource={processes} 
        treeColumnIndex={0} childMapping='children'
        allowFiltering={true} filterSettings={filterSettings}
        allowSorting={true} 
        allowResizing={true} 
        gridLines='Both' 
    >
        <ColumnsDirective>
            <ColumnDirective field='name' headerText='Process Name'/>
            <ColumnDirective field='pid' headerText='PID'/>
            <ColumnDirective field='ppid' headerText='PPID' />
            <ColumnDirective field='state' headerText='State' />
            <ColumnDirective field='vmsize' headerText='VM Size' />
            <ColumnDirective field='nice' headerText='Nice' />
            <ColumnDirective field='cpu_time' headerText='CPU Time'/>
        </ColumnsDirective>
        <Inject services={[Resize, Sort, Filter]}/>
    </TreeGridComponent>;
};


export default App;

