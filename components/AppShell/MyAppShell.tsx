import React from 'react';
import {AppShell, Header, Image} from '@mantine/core';
import LightAndDarkModeButton from "../LightAndDarkModeButton";
import MainLinks from "./_mainLinks";

export default function MyAppShell({children}): JSX.Element {
    return (<AppShell
        navbarOffsetBreakpoint="sm"
        header={
            <Header height={140} p="md">
                <div style={{display: 'flex', alignItems: 'center', height: '100%'}}>
                    <div style={{marginRight: '1em'}}>
                        <Image src={"https://acspa.ro/wp-content/uploads/2020/04/cropped-ACS-dd-oval-400-190x127.png"}
                               alt="ACS Perpetuum Activ"/>
                    </div>

                    <div style={{marginLeft: 'auto'}}>
                        <LightAndDarkModeButton/>
                    </div>

                    <div style={{marginLeft: '1em', marginRight: '1em'}}>
                        <MainLinks/>
                    </div>
                </div>
            </Header>
        }
    >
        {children}
    </AppShell>);
}
